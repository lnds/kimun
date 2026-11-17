use std::path::PathBuf;

use clap::Args;

/// Arguments of `km deps` besides the common ones.
#[derive(Args)]
pub struct DepsArgs {
    /// Show only files with dependency cycles
    #[arg(long)]
    pub cycles_only: bool,

    /// Sort by: fan-out (default), fan-in
    #[arg(long, default_value = "fan-out", value_parser = ["fan-out", "fan-in"])]
    pub sort_by: String,

    /// Rows of the table (default: 20); JSON lists every one of the files unless given
    #[arg(long)]
    pub top: Option<usize>,

    /// Print the dependency structure matrix instead of the table
    #[arg(long, conflicts_with = "top")]
    pub matrix: bool,

    /// Levels of directories the matrix shows expanded (default: 1)
    #[arg(long, requires = "matrix", value_parser = clap::value_parser!(u32).range(1..))]
    pub depth: Option<u32>,

    /// Expand this directory of the matrix, and the ones above it, alone
    #[arg(long, requires = "matrix", conflicts_with = "depth")]
    pub focus: Option<String>,

    /// Write the matrix as an interactive HTML page to this file
    #[arg(long, value_name = "FILE", requires = "matrix")]
    pub html: Option<PathBuf>,
}
