//! Running a command: the target it measures, what `--list-excluded`
//! prints, and the error it ends with.

use std::path::PathBuf;

use crate::report_helpers;
use crate::walk::{self, ExcludeFilter};

/// Resolve an optional path to a default of "." and run an analysis
/// command, printing errors to stderr and exiting with code 1 on failure.
pub fn run_command(
    path: Option<PathBuf>,
    f: impl FnOnce(&std::path::Path) -> Result<(), Box<dyn std::error::Error>>,
) {
    let target = path.unwrap_or_else(|| PathBuf::from("."));
    report_helpers::set_path_base(&target);
    if let Err(err) = f(&target) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

/// If `--list-excluded` was passed, print the excluded files and exit.
pub fn maybe_list_excluded(
    path: &Option<PathBuf>,
    include_tests: bool,
    filter: &ExcludeFilter,
    list_excluded: bool,
) {
    if !list_excluded {
        return;
    }
    let target = path.as_deref().unwrap_or(std::path::Path::new("."));
    if let Err(err) = walk::print_excluded_files(target, !include_tests, filter) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
    std::process::exit(0);
}
