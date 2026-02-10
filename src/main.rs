mod cycom;
mod dups;
mod git;
mod hal;
mod hotspots;
mod indent;
mod loc;
mod mi;
mod miv;
mod report;
mod util;
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

    /// Analyze Halstead complexity metrics per file
    #[command(long_about = "\
Analyze Halstead complexity metrics per file.

Halstead metrics measure software complexity based on operators and operands
extracted from source code.

Base counts:
  n1 = distinct operators    n2 = distinct operands
  N1 = total operators       N2 = total operands

Derived metrics:
  Vocabulary (n) = n1 + n2
  Length (N)     = N1 + N2
  Volume (V)     = N * log2(n)       -- size of the implementation
  Difficulty (D) = (n1/2) * (N2/n2)  -- error proneness
  Effort (E)     = D * V             -- mental effort to develop
  Bugs (B)       = V / 3000          -- estimated delivered bugs
  Time (T)       = E / 18 seconds    -- estimated development time

Higher effort/volume/bugs indicate more complex and error-prone code.")]
    Hal {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: effort, volume, or bugs (default: effort)
        #[arg(long, default_value = "effort", value_parser = ["effort", "volume", "bugs"])]
        sort_by: String,
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

        /// Sort by metric: total, max, or avg (default: total)
        #[arg(long, default_value = "total", value_parser = ["total", "max", "avg"])]
        sort_by: String,
    },

    /// Compute Maintainability Index per file (Visual Studio variant, 0-100 scale)
    #[command(long_about = "\
Compute Maintainability Index (MI) per file using the Visual Studio variant.

This variant normalizes MI to a 0-100 scale with no comment-weight term.
For the verifysoft variant (with comment weight), use `cm miv` instead.

Formula:
  MI = MAX(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)) * 100 / 171)

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines.

Thresholds:
  20-100  green   -- good maintainability
  10-19   yellow  -- moderate maintainability
  0-9     red     -- low maintainability")]
    Mi {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Generate a comprehensive report combining all code metrics
    Report {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files per section (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,

        /// Show all files instead of truncating to top N
        #[arg(long)]
        full: bool,
    },

    /// Compute Maintainability Index per file (verifysoft variant, with comment weight)
    #[command(long_about = "\
Compute Maintainability Index (MI) per file using the verifysoft.com variant.

This variant includes a comment-weight term (MIcw) that rewards well-commented
code. For the simpler Visual Studio variant (0-100 scale, no comment weight),
use `cm mi` instead.

Formula:
  MIwoc = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
  MIcw  = 50 * sin(sqrt(2.46 * radians(PerCM)))
  MI    = MIwoc + MIcw

Where V = Halstead Volume, G = cyclomatic complexity,
LOC = code lines, PerCM = comment percentage (converted to radians).

Thresholds:
  85+     good         -- easy to maintain
  65-84   moderate     -- reasonable maintainability
  <65     difficult    -- hard to maintain")]
    Miv {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Find hotspots: files that change frequently and have high complexity
    #[command(long_about = "\
Find hotspots: files that change frequently AND have high complexity.

Based on Adam Thornhill's method (\"Your Code as a Crime Scene\"):
  Score = commits × cyclomatic complexity

Files with high scores are both change-prone and complex — they concentrate
risk and are the highest-value refactoring targets.

Requires a git repository. Use --since to limit the analysis window
(approximations: 1 month = 30 days, 1 year = 365 days).

Examples:
  cm hotspots              # all history, top 20 by score
  cm hotspots --since 6m   # last 6 months
  cm hotspots --since 1y --sort-by commits
  cm hotspots --top 10     # show only top 10
  cm hotspots --json       # machine-readable output")]
    Hotspots {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: score, commits, or complexity (default: score)
        #[arg(long, default_value = "score", value_parser = ["score", "commits", "complexity"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,
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
        Commands::Hal {
            path,
            json,
            include_tests,
            top,
            sort_by,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = hal::run(&target, json, include_tests, top, &sort_by) {
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
            sort_by,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = cycom::run(
                &target,
                json,
                include_tests,
                min_complexity,
                top,
                per_function,
                &sort_by,
            ) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Mi {
            path,
            json,
            include_tests,
            top,
            sort_by,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = mi::run(&target, json, include_tests, top, &sort_by) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Report {
            path,
            json,
            include_tests,
            top,
            min_lines,
            full,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            let effective_top = if full { usize::MAX } else { top };
            if let Err(err) = report::run(&target, json, include_tests, effective_top, min_lines) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Miv {
            path,
            json,
            include_tests,
            top,
            sort_by,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = miv::run(&target, json, include_tests, top, &sort_by) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Commands::Hotspots {
            path,
            json,
            include_tests,
            top,
            sort_by,
            since,
        } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = hotspots::run(
                &target,
                json,
                include_tests,
                top,
                &sort_by,
                since.as_deref(),
            ) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
    }
}
