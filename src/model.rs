use crate::data::ProcessedLapData;
use linfa::prelude::*;
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2};
pub type FittedLinearRegression = linfa_linear::FittedLinearRegression<f64>;

pub struct DegradationModel {
    pub hard_model: Option<FittedLinearRegression>,
    pub medium_model: Option<FittedLinearRegression>,
}

impl DegradationModel {
    pub fn new(laps: &[ProcessedLapData]) -> Self {
        let data: Vec<_> = laps.iter()
            .filter(|d| !d.is_pit_out_lap && !d.is_pit_in_lap && d.time_delta.is_finite())
            .collect();
        Self {
            hard_model: Self::build_model(&data, "HARD"),
            medium_model: Self::build_model(&data, "MEDIUM"),
        }
    }

    fn build_model(data: &[&ProcessedLapData], comp: &str) -> Option<FittedLinearRegression> {
         let comp_data: Vec<_> = data.iter().filter(|d| d.compound == comp).collect();
        if comp_data.len() < 5 { return None; }

        let feats: Vec<f64> = comp_data.iter()
            .flat_map(|d| vec![d.tyre_life as f64, d.track_temp, (d.tyre_life as f64).powi(2)])
            .collect();
        let targets: Vec<f64> = comp_data.iter().map(|d| d.time_delta).collect();

        let x = Array2::from_shape_vec((comp_data.len(), 3), feats).ok()?;
        let y = Array1::from_vec(targets);
        let ds = Dataset::new(x, y);

        LinearRegression::new().fit(&ds).ok()
    }

    pub fn predict_degradation(&self, tyre_lap: u32, temp: f64, comp: &str) -> f64 {
        let model = match comp.to_uppercase().as_str() {
            "HARD" => self.hard_model.as_ref(),
            "MEDIUM" => self.medium_model.as_ref(),
            _ => return 0.0,
        };

        model.map_or(0.0, |m| {
            let feats = Array1::from_vec(vec![tyre_lap as f64, temp, (tyre_lap as f64).powi(2)]);
            m.predict(&feats.into_shape((1, 3)).expect("Shape error"))[0].max(0.0)
        })
    }
}