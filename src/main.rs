mod cycom;
mod dups;
mod indent;
mod loc;
mod walk;

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

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,
    },

    /// Analyze indentation complexity (stddev and max depth per file)
    Indent {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,
    },

    /// Analyze cyclomatic complexity per file and per function
    Cycom {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Minimum max-complexity to include a file (default: 1)
        #[arg(long, default_value = "1")]
        min_complexity: usize,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Show per-function breakdown
        #[arg(long)]
        per_function: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc {
            path,
            verbose,
            json,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = loc::run(&target, verbose, json) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Dups {
            path,
            report,
            show_all,
            min_lines,
            json,
            include_tests,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = dups::run(&target, min_lines, report, show_all, json, !include_tests)
            {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Indent {
            path,
            json,
            include_tests,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = indent::run(&target, json, include_tests) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Cycom {
            path,
            json,
            include_tests,
            min_complexity,
            top,
            per_function,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = cycom::run(
                &target,
                json,
                include_tests,
                min_complexity,
                top,
                per_function,
            ) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
    }
}
