use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use csv::ReaderBuilder;
use std::cmp::Ordering;

// renaming all the columns we need to use in the project
#[derive(Debug, Deserialize)]
struct RawLapData {
    #[serde(rename = "Driver")] driver: String,
    #[serde(rename = "LapNumber")] lap_number: f64,
    #[serde(rename = "Compound_lap")] compound: String,
    #[serde(rename = "TyreLife")] tyre_life: f64,
    #[serde(rename = "LapTimeSeconds_lap")] lap_time_seconds: f64,
    #[serde(rename = "TrackTemp")] track_temp: f64,
    #[serde(rename = "PitOutTime")] pit_out_time: Option<String>,
    #[serde(rename = "PitInTime")] pit_in_time: Option<Option<String>>,
}

// defining struct for clarity 
#[derive(Debug, Clone)]
pub struct ProcessedLapData {
    pub driver: String,
    pub lap_number: u32,
    pub compound: String,
    pub tyre_life: u32,
    pub lap_time_seconds: f64,
    pub track_temp: f64,
    pub time_delta: f64,
    pub is_pit_out_lap: bool,
    pub is_pit_in_lap: bool,
}

// the following is the DataProcessor struct and related function. It would re-organize data followed by driver name. 
// the original dataset is lap-based rather than driver-based, therefore we need to make some changes here.
pub struct DataProcessor {
    pub driver_data: HashMap<String, Vec<ProcessedLapData>>,
}

impl DataProcessor {
    pub fn new<P: AsRef<Path>>(filename: P) -> Result<Self, Box<dyn Error>> {
        let mut reader = ReaderBuilder::new().has_headers(true).from_path(filename)?;
        let mut driver_data_unsorted: HashMap<String, Vec<ProcessedLapData>> = HashMap::new();

        for res in reader.deserialize() {
            let raw: RawLapData = res?;
            if raw.lap_time_seconds > 0.0 && raw.lap_time_seconds < 300.0 && raw.track_temp > 0.0 &&
               raw.tyre_life >= 0.0 && raw.lap_number >= 1.0 && !raw.driver.is_empty() && !raw.compound.is_empty()
            {
                let lap = ProcessedLapData {
                    driver: raw.driver,
                    lap_number: raw.lap_number.round() as u32,
                    compound: raw.compound.to_uppercase(),
                    tyre_life: raw.tyre_life.round() as u32,
                    lap_time_seconds: raw.lap_time_seconds,
                    track_temp: raw.track_temp,
                    time_delta: 0.0,
                    is_pit_out_lap: raw.pit_out_time.is_some(),
                    is_pit_in_lap: raw.pit_in_time.flatten().is_some(),
                };
                // Pushing 'lap' into the vector within the HashMap moves its content, including 'lap.driver'
                driver_data_unsorted.entry(lap.driver.clone()).or_default().push(lap);
            }
        }

        let mut driver_data = HashMap::new();
        // When iterating here, 'driver' (the String key) is moved out of driver_data_unsorted
        for (driver, mut laps) in driver_data_unsorted {
            laps.sort_by_key(|d| d.lap_number);
            let min_lap = laps.iter()
                .filter(|d| !d.is_pit_in_lap && !d.is_pit_out_lap)
                .map(|d| d.lap_time_seconds)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

            let delta = min_lap.unwrap_or(0.0);
            laps.iter_mut().for_each(|lap| lap.time_delta = lap.lap_time_seconds - delta);

            // FIX: Clone the driver String before inserting it into the new HashMap, or there's going to be an issue here. 
            // it took me a long time to find out this wrong line, though...
            driver_data.insert(driver.clone(), laps);
        }
        Ok(DataProcessor { driver_data })
    }
}


// calculating average stint length based on the "pit-in" and "pit-out" columns in the dataset. 
// originally, I only tracked "tyre" column: iterating through the tyre column, if there's a change in compound type, that is a pit-stop.
// but this is not accurate and unrealistic at all! What if a driver used two sets of medium, and did not change the compound type? 
// therefore, using pit-in and pit-out should be a better approach. 
pub fn calculate_average_stint_lengths(data: &HashMap<String, Vec<ProcessedLapData>>) -> (f64, f64) {
    let mut medium_stints = Vec::new();
    let mut hard_stints = Vec::new();

    for laps in data.values().filter(|l| !l.is_empty()) {
        let mut start_lap = laps[0].lap_number;
        let mut compound = laps[0].compound.clone();

        for i in 0..laps.len() {
            let curr = &laps[i];
            let is_last = i == laps.len() - 1;
            let next_is_pit_out = laps.get(i + 1).map_or(false, |nl| nl.is_pit_out_lap);
            let next_compound = laps.get(i + 1).map(|nl| &nl.compound);
            let next_exists = i + 1 < laps.len();

            let is_stint_end = is_last || curr.is_pit_in_lap || next_is_pit_out ||
                               (next_exists && next_compound.map_or(false, |nc| *nc != curr.compound) && !next_is_pit_out);

            if is_stint_end {
                let len = curr.lap_number - start_lap + 1;
                if len > 0 {
                     match compound.as_str() {
                        "MEDIUM" => medium_stints.push(len),
                        "HARD" => hard_stints.push(len),
                        _ => {}
                    }
                }
                if !is_last {
                     let next = &laps[i + 1];
                     start_lap = next.lap_number;
                     compound = next.compound.clone();
                }
            }
        }
    }

    let avg = |stints: &Vec<u32>, default: f64| {
        if stints.is_empty() { default } else { stints.iter().sum::<u32>() as f64 / stints.len() as f64 }
    };

    (avg(&medium_stints, 14.0), avg(&hard_stints, 42.0))
}

/* 
#[cfg(test)]
mod stint_length_tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_data() -> HashMap<String, Vec<ProcessedLapData>> {
        let mut driver_data = HashMap::new();
        
        // Driver 1: 5-lap MEDIUM stint, 4-lap HARD stint
        driver_data.insert("DRIVER1".to_string(), vec![
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 1,
                compound: "MEDIUM".to_string(),
                tyre_life: 0,
                lap_time_seconds: 100.0,
                track_temp: 30.0,
                time_delta: 1.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 2,
                compound: "MEDIUM".to_string(),
                tyre_life: 1,
                lap_time_seconds: 101.0,
                track_temp: 30.0,
                time_delta: 2.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 3,
                compound: "MEDIUM".to_string(),
                tyre_life: 2,
                lap_time_seconds: 99.0,
                track_temp: 30.0,
                time_delta: 0.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 4,
                compound: "MEDIUM".to_string(),
                tyre_life: 3,
                lap_time_seconds: 102.0,
                track_temp: 30.0,
                time_delta: 3.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 5,
                compound: "MEDIUM".to_string(),
                tyre_life: 4,
                lap_time_seconds: 103.0,
                track_temp: 30.0,
                time_delta: 4.0,
                is_pit_out_lap: false,
                is_pit_in_lap: true, // Pit in
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 6,
                compound: "HARD".to_string(),
                tyre_life: 0,
                lap_time_seconds: 105.0,
                track_temp: 30.0,
                time_delta: 6.0,
                is_pit_out_lap: true, // Pit out
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 7,
                compound: "HARD".to_string(),
                tyre_life: 1,
                lap_time_seconds: 104.0,
                track_temp: 30.0,
                time_delta: 5.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 8,
                compound: "HARD".to_string(),
                tyre_life: 2,
                lap_time_seconds: 106.0,
                track_temp: 30.0,
                time_delta: 7.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER1".to_string(),
                lap_number: 9,
                compound: "HARD".to_string(),
                tyre_life: 3,
                lap_time_seconds: 107.0,
                track_temp: 30.0,
                time_delta: 8.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
        ]);

        // Driver 2: 3-lap MEDIUM stint, 2-lap HARD stint
        driver_data.insert("DRIVER2".to_string(), vec![
            ProcessedLapData {
                driver: "DRIVER2".to_string(),
                lap_number: 1,
                compound: "MEDIUM".to_string(),
                tyre_life: 0,
                lap_time_seconds: 95.0,
                track_temp: 30.0,
                time_delta: 0.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER2".to_string(),
                lap_number: 2,
                compound: "MEDIUM".to_string(),
                tyre_life: 1,
                lap_time_seconds: 96.0,
                track_temp: 30.0,
                time_delta: 1.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER2".to_string(),
                lap_number: 3,
                compound: "MEDIUM".to_string(),
                tyre_life: 2,
                lap_time_seconds: 97.0,
                track_temp: 30.0,
                time_delta: 2.0,
                is_pit_out_lap: false,
                is_pit_in_lap: true, // Pit in
            },
            ProcessedLapData {
                driver: "DRIVER2".to_string(),
                lap_number: 4,
                compound: "HARD".to_string(),
                tyre_life: 0,
                lap_time_seconds: 98.0,
                track_temp: 30.0,
                time_delta: 3.0,
                is_pit_out_lap: true, // Pit out
                is_pit_in_lap: false,
            },
            ProcessedLapData {
                driver: "DRIVER2".to_string(),
                lap_number: 5,
                compound: "HARD".to_string(),
                tyre_life: 1,
                lap_time_seconds: 99.0,
                track_temp: 30.0,
                time_delta: 4.0,
                is_pit_out_lap: false,
                is_pit_in_lap: false,
            },
        ]);

        driver_data
    }

    #[test]
    fn test_average_stint_length_calculation() {
        let test_data = create_test_data();
        let (avg_medium, avg_hard) = calculate_average_stint_lengths(&test_data);
        
        // DRIVER1: 5-lap MEDIUM, 4-lap HARD
        // DRIVER2: 3-lap MEDIUM, 2-lap HARD
        assert_eq!(avg_medium, (5.0 + 3.0) / 2.0);
        assert_eq!(avg_hard, (4.0 + 2.0) / 2.0);
    }
}
*/ 
