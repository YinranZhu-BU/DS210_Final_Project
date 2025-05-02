mod data;
mod model;
mod strategy;

use data::{DataProcessor, calculate_average_stint_lengths, evaluate_model_accuracy};
use model::DegradationModel;
use strategy::StrategySimulator;
use std::error::Error; 

// The main entry point of the application.
// It now returns a Result to allow using the '?' operator for concise error handling.
fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting Simulation...");

    // Load and process data from the CSV file.
    let processor = DataProcessor::new("Shanghai2025.csv")?; // Use '?' to propagate errors

    // Check if the data loading resulted in any valid driver data.
    if processor.driver_data.is_empty() {
        return Err("No data found.".into()); // Return an error if no data is found
    }

    // Calculate the average stint lengths for medium and hard compounds.
    let (avg_med, avg_hard) = calculate_average_stint_lengths(&processor.driver_data);
    println!("Avg Stints - Med: {:.1}, Hard: {:.1}", avg_med, avg_hard);

    // Prepare data for the degradation model: flatten all laps and filter out pit laps and non-finite time deltas.
    let model_laps: Vec<data::ProcessedLapData> = processor.driver_data.values()
        .cloned().flatten()
        .filter(|d| !d.is_pit_out_lap && !d.is_pit_in_lap && d.time_delta.is_finite())
        .collect();

    // Build the degradation models for each compound using the filtered data.
    let model = DegradationModel::new(&model_laps);

    // Check if at least one degradation model was successfully built.
    // If neither model could be built, return an error as simulation requires models.
    if model.medium_model.is_none() && model.hard_model.is_none() {
         return Err("Failed to build any models.".into()); // Return an error if no models are built
    }

    // Evaluate model accuracy
    let (med_error, hard_error) = evaluate_model_accuracy(&model, &processor.driver_data);
    println!("\nModel Prediction Accuracy (Mean Absolute Error):");
    println!("- Medium compound: {:.3} seconds per lap", med_error);
    println!("- Hard compound: {:.3} seconds per lap", hard_error);
    
    // Run the strategy simulation using the built models and average stint lengths.
    StrategySimulator::simulate_and_print(&model, avg_med, avg_hard)?; // Use '?' to propagate errors

    // Return Ok(()) to indicate successful execution if no errors occurred.
    Ok(())
}

