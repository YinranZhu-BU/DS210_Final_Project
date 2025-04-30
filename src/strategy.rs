use crate::model::DegradationModel;
use std::error::Error;

pub struct StrategySimulator;
// simulate strategies based on the linear regression model.
impl StrategySimulator {
    pub fn simulate_and_print(
        model: &DegradationModel,
        avg_med: f64,
        avg_hard: f64,
    ) -> Result<(), Box<dyn Error>> {
        let pit_loss = 21.0;
        let temp = 32.0;
        let total_laps: u32 = 56;

        println!("\n--- Strategies (Laps: {}, Pit Loss: {}s) ---", total_laps, pit_loss);

        let avg_med_u32 = avg_med.round() as u32;
        let avg_hard_u32 = avg_hard.round() as u32;

        let strategies = vec![
            ("1S (M-H)", { let s1=avg_med_u32; let s2=total_laps.saturating_sub(s1); if s1>0&&s2>0&&s1+s2==total_laps { Some(vec![("MEDIUM",s1),("HARD",s2)])}else{None} }),
            ("1S (H-M)", { let s1=avg_hard_u32; let s2=total_laps.saturating_sub(s1); if s1>0&&s2>0&&s1+s2==total_laps { Some(vec![("HARD",s1),("MEDIUM",s2)])}else{None} }),
            ("1S (H-H)", { let s1=avg_hard_u32; let s2=total_laps.saturating_sub(s1); if s1>0&&s2>0&&s1+s2==total_laps { Some(vec![("HARD",s1),("HARD",s2)])}else{None} }),
            ("2S (M-H-H)", { let s1=avg_med_u32; let rem=total_laps.saturating_sub(s1); let s2=(rem as f64/2.0).round() as u32; let s3=rem.saturating_sub(s2); if s1>0&&s2>0&&s3>0&&s1+s2+s3==total_laps{Some(vec![("MEDIUM",s1),("HARD",s2),("HARD",s3)])}else{None} }),
            ("2S (H-M-M)", { let s1=avg_hard_u32; let rem=total_laps.saturating_sub(s1); let s2=(rem as f64/2.0).round() as u32; let s3=rem.saturating_sub(s2); if s1>0&&s2>0&&s3>0&&s1+s2+s3==total_laps{Some(vec![("HARD",s1),("MEDIUM",s2),("MEDIUM",s3)])}else{None} }),
        ];

        for (name, stints_opt) in strategies {
             if let Some(stints) = stints_opt {
                let total_laps_strat: u32 = stints.iter().map(|(_, l)| l).sum();
                if total_laps_strat != total_laps || stints.iter().any(|(_, l)| *l == 0) { continue; }

                let mut total_delta = 0.0;
                let mut pits = 0;

                for (i, (comp, num_laps)) in stints.iter().enumerate() {
                    if i > 0 { total_delta += pit_loss; pits += 1; }
                    let stint_delta: f64 = (1..=*num_laps)
                        .map(|tyre_lap| model.predict_degradation(tyre_lap, temp, comp))
                        .sum();
                    total_delta += stint_delta;
                }
                println!("- {:10} : {:6.2}s ({} stops)", name, total_delta, pits);
             }
        }
        Ok(())
    }
}

