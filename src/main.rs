mod dups;
mod loc;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cm", about = "Code metrics tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Count lines of code (blank, comment, code) by language
    Loc {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Show summary stats (files read, unique, ignored, elapsed time)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Detect duplicate code across files
    Dups {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Show detailed report with duplicate locations
        #[arg(short, long)]
        report: bool,

        /// Show all duplicate groups (default: top 20)
        #[arg(long)]
        show_all: bool,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc { path, verbose } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = loc::run(&target, verbose) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Dups {
            path,
            report,
            show_all,
            min_lines,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = dups::run(&target, min_lines, report, show_all) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
    }
}
