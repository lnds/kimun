use std::error::Error;

use super::ProjectReport;

pub fn print_json(report: &ProjectReport) -> Result<(), Box<dyn Error>> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}
