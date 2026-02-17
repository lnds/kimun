use std::error::Error;

use super::ProjectReport;
use crate::report_helpers;

pub fn print_json(report: &ProjectReport) -> Result<(), Box<dyn Error>> {
    report_helpers::print_json_stdout(report)
}
