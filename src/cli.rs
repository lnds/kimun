/// CLI argument definitions for the `km` command.
///
/// Defines all subcommands and their arguments using the `clap` derive macros.
/// Long help text is stored in `cli_help.rs` to keep this file focused on structure.
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::cli_help;

/// Top-level CLI parser with a single subcommand selector.
#[derive(Parser)]
#[command(name = "km", version, about = "Kimün — code metrics tools")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Common arguments shared by most analysis commands.
#[derive(Args)]
pub struct CommonArgs {
    /// Directory to analyze (default: current directory)
    pub path: Option<PathBuf>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Include test files and directories in analysis (excluded by default)
    #[arg(long)]
    pub include_tests: bool,
}

/// All available analysis subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Count lines of code (blank, comment, code) by language
    Loc {
        #[command(flatten)]
        common: CommonArgs,

        /// Show summary stats (files read, unique, ignored, elapsed time)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Detect duplicate code across files
    Dups {
        #[command(flatten)]
        common: CommonArgs,

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

    /// Analyze indentation complexity (stddev and max depth per file)
    Indent {
        #[command(flatten)]
        common: CommonArgs,
    },

    /// Analyze Halstead complexity metrics per file
    #[command(long_about = cli_help::HAL)]
    Hal {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: effort, volume, or bugs (default: effort)
        #[arg(long, default_value = "effort", value_parser = ["effort", "volume", "bugs"])]
        sort_by: String,
    },

    /// Analyze cyclomatic complexity per file and per function
    Cycom {
        #[command(flatten)]
        common: CommonArgs,

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

    /// Analyze cognitive complexity per file and per function (SonarSource method)
    #[command(long_about = cli_help::COGCOM)]
    Cogcom {
        #[command(flatten)]
        common: CommonArgs,

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
    #[command(long_about = cli_help::MI)]
    Mi {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Generate a comprehensive report combining all code metrics
    Report {
        #[command(flatten)]
        common: CommonArgs,

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
    #[command(long_about = cli_help::MIV)]
    Miv {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Find hotspots: files that change frequently and have high complexity
    #[command(long_about = cli_help::HOTSPOTS)]
    Hotspots {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: score, commits, or complexity (default: score)
        #[arg(long, default_value = "score", value_parser = ["score", "commits", "complexity"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Complexity metric: indent (default, Thornhill), cycom (cyclomatic), or cogcom (cognitive)
        #[arg(long, default_value = "indent", value_parser = ["indent", "cycom", "cogcom"])]
        complexity: String,
    },

    /// Analyze code ownership patterns via git blame (knowledge maps)
    #[command(long_about = cli_help::KNOWLEDGE)]
    Knowledge {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by: concentration, diffusion, or risk (default: concentration)
        #[arg(long, default_value = "concentration", value_parser = ["concentration", "diffusion", "risk"])]
        sort_by: String,

        /// Only consider recent activity since this time for knowledge loss detection (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Show only files with knowledge loss risk (primary owner inactive)
        #[arg(long)]
        risk_only: bool,
    },

    /// Analyze temporal coupling: files that change together in commits
    #[command(long_about = cli_help::TC)]
    Tc {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N file pairs (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by: strength or shared (default: strength)
        #[arg(long, default_value = "strength", value_parser = ["strength", "shared"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Minimum commits per file to be included (default: 3)
        #[arg(long, default_value = "3")]
        min_degree: usize,

        /// Filter results: show only pairs with strength >= threshold (e.g. 0.5 for strong coupling only)
        #[arg(long)]
        min_strength: Option<f64>,
    },

    /// Compute an overall code health score for the project (A++ to F--)
    #[command(long_about = cli_help::SCORE)]
    Score {
        #[command(subcommand)]
        subcommand: Option<ScoreCommands>,

        #[command(flatten)]
        common: CommonArgs,

        /// Number of worst files to show in "needs attention" (default: 10)
        #[arg(long, default_value = "10")]
        bottom: usize,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,

        /// Scoring model: cogcom (default, v0.14+) or legacy (MI + cyclomatic, v0.13)
        #[arg(long, default_value = "cogcom", value_parser = ["cogcom", "legacy"])]
        model: String,
    },

    /// AI-powered code analysis and tooling
    Ai {
        #[command(subcommand)]
        command: AiCommands,
    },
}

/// Score subcommands (diff).
#[derive(Subcommand)]
pub enum ScoreCommands {
    /// Compare the current code health score against a git ref
    #[command(long_about = cli_help::SCORE_DIFF)]
    Diff {
        /// Git ref to compare against (default: HEAD)
        #[arg(long, value_name = "REF", default_value = "HEAD")]
        git_ref: String,

        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Number of worst files to show in "needs attention" (default: 10)
        #[arg(long, default_value = "10")]
        bottom: usize,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,

        /// Scoring model: cogcom (default, v0.14+) or legacy (MI + cyclomatic, v0.13)
        #[arg(long, default_value = "cogcom", value_parser = ["cogcom", "legacy"])]
        model: String,
    },
}

/// AI-powered analysis subcommands (analyze, skill install).
#[derive(Subcommand)]
pub enum AiCommands {
    /// Analyze repository using an AI provider
    #[command(long_about = cli_help::AI_ANALYZE)]
    Analyze {
        /// AI provider to use (e.g. claude)
        provider: String,

        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Model to use (default: claude-sonnet-4-5-20250929)
        #[arg(long)]
        model: Option<String>,

        /// Save the report to a file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Install a Claude Code skill for km
    #[command(long_about = cli_help::AI_SKILL)]
    Skill {
        /// Provider for the skill (e.g. claude)
        provider: String,
    },
}
