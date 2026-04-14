/// CLI argument definitions for the `km` command.
///
/// Defines all subcommands and their arguments using the `clap` derive macros.
/// Long help text is stored in `cli_help.rs` to keep this file focused on structure.
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
pub use clap_complete::Shell;

use crate::cli_help;
use crate::walk::ExcludeFilter;

/// Error returned by commands that do not support CI annotation formats.
pub const ERR_CI_FORMAT_ONLY: &str =
    "--format github and --format codeclimate are only supported by cycom, cogcom, and smells";

/// Output format for analysis commands.
///
/// Driven by `--format` on `CommonArgs`. `Github` and `Codeclimate` emit CI
/// annotations and are only supported by cycom, cogcom, and smells.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, ValueEnum)]
pub enum OutputMode {
    /// Human-readable table (default)
    #[default]
    Table,
    /// Machine-readable JSON
    Json,
    /// Single compact line of key:value pairs (AI-friendly)
    Short,
    /// Single headline metric value (for piping/embedding)
    Terse,
    /// GitHub Actions warning annotations (CI)
    Github,
    /// CodeClimate JSON report (GitLab Code Quality, Codacy, etc.)
    /// Also accepted as `--format gitlab` for discoverability.
    #[value(alias = "gitlab")]
    Codeclimate,
}

/// Top-level CLI parser with a single subcommand selector.
#[derive(Parser)]
#[command(name = "km", version, about = "Kimün — code metrics tools")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Exclude-filter arguments shared by all commands that walk the filesystem.
#[derive(Args)]
pub struct ExcludeArgs {
    /// Only include files with these extensions (case-insensitive, leading dot optional).
    /// When set, all other extensions are excluded. Cannot be combined with --exclude-ext.
    /// Repeatable: --include-ext rs --include-ext toml
    #[arg(long, value_name = "EXT", conflicts_with = "exclude_ext")]
    pub include_ext: Vec<String>,

    /// Exclude files by extension, case-insensitive (leading dot optional: "js" and ".js" are equivalent).
    /// Repeatable: --exclude-ext js --exclude-ext ts
    #[arg(long, value_name = "EXT")]
    pub exclude_ext: Vec<String>,

    /// Exclude directories by exact name (case-sensitive).
    /// Matches directory names at any depth in the tree.
    /// Repeatable: --exclude-dir vendor --exclude-dir dist
    #[arg(long, value_name = "DIR")]
    pub exclude_dir: Vec<String>,

    /// Exclude files matching a glob pattern against the relative path from the analysis root.
    /// Use this for compound extensions (e.g. "*.min.js") or path patterns (e.g. "vendor/**").
    /// For simple extensions prefer --exclude-ext; for directory names prefer --exclude-dir.
    /// Repeatable: --exclude "*.min.js" --exclude "generated/**"
    #[arg(long, short = 'E', value_name = "PATTERN")]
    pub exclude: Vec<String>,

    /// Print files that would be excluded by the current filter and exit.
    /// Useful for debugging filter rules before running an analysis.
    #[arg(long)]
    pub list_excluded: bool,
}

impl ExcludeArgs {
    /// Build an `ExcludeFilter` from the parsed CLI flags.
    pub fn exclude_filter(&self) -> ExcludeFilter {
        ExcludeFilter::new(
            &self.include_ext,
            &self.exclude_ext,
            &self.exclude_dir,
            &self.exclude,
        )
    }

    /// Returns `true` if no exclude/include flags were specified.
    pub fn is_empty(&self) -> bool {
        self.include_ext.is_empty()
            && self.exclude_ext.is_empty()
            && self.exclude_dir.is_empty()
            && self.exclude.is_empty()
    }
}

/// Common arguments shared by most analysis commands.
#[derive(Args)]
pub struct CommonArgs {
    /// Directory to analyze (default: current directory)
    pub path: Option<PathBuf>,

    /// Output format: table (default), json, short, terse, or github.
    /// `short` emits a single compact line of key:value pairs (AI-friendly).
    /// `terse` emits a single headline metric value (for piping/embedding).
    /// `github` emits GitHub Actions warning annotations (only cycom, cogcom, smells).
    #[arg(long, value_enum, default_value_t)]
    pub format: OutputMode,

    /// Include test files and directories in analysis (excluded by default)
    #[arg(long)]
    pub include_tests: bool,

    #[command(flatten)]
    pub exclude_args: ExcludeArgs,
}

impl CommonArgs {
    /// Build an `ExcludeFilter` from the `--exclude-ext`, `--exclude-dir`, and `--exclude` flags.
    pub fn exclude_filter(&self) -> ExcludeFilter {
        self.exclude_args.exclude_filter()
    }

    /// Whether `--list-excluded` was requested.
    pub fn list_excluded(&self) -> bool {
        self.exclude_args.list_excluded
    }
}

/// All available analysis subcommands.
#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // CLI args are parsed once; size is not performance-critical
pub enum Commands {
    /// Count lines of code (blank, comment, code) by language
    Loc {
        #[command(flatten)]
        common: CommonArgs,

        /// Show summary stats (files read, unique, ignored, elapsed time)
        #[arg(short, long)]
        verbose: bool,

        /// Break down lines of code by git author (requires a git repository)
        #[arg(long)]
        by_author: bool,
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

        /// Exit with code 1 if duplicate groups exceed this limit.
        /// Useful as a CI quality gate: --max-duplicates 0 fails on any duplicate.
        #[arg(long, value_name = "N")]
        max_duplicates: Option<usize>,

        /// Exit with code 1 if the duplicated-lines ratio exceeds this percentage.
        /// Useful for ratcheting down duplication over time: --max-dup-ratio 5.0
        /// fails when more than 5% of code lines are duplicated.
        #[arg(long, value_name = "PERCENT")]
        max_dup_ratio: Option<f64>,

        /// Exit with code 1 if the current duplication ratio is higher than at the
        /// given git ref. Prevents duplication debt from growing silently in CI:
        /// --fail-on-increase origin/main fails if this branch added more duplicates.
        #[arg(long, value_name = "REF")]
        fail_on_increase: Option<String>,
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

    /// Analyze code churn: pure change frequency per file (git commits only)
    Churn {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by: commits (default), rate (commits/month), or file
        #[arg(long, default_value = "commits", value_parser = ["commits", "rate", "file"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,
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

        /// Aggregate by author: files owned, lines, languages, worst risk
        #[arg(long)]
        summary: bool,

        /// Compute the project bus factor: the minimum number of contributors
        /// whose combined ownership covers 80% of the code. A bus factor of 1
        /// means one person holds most knowledge — extremely high risk.
        #[arg(long)]
        bus_factor: bool,

        /// Show only files owned by this author (substring match, case-insensitive)
        #[arg(long, value_name = "NAME")]
        author: Option<String>,
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

    /// Detect common code smells per file
    #[command(long_about = cli_help::SMELLS)]
    Smells {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Maximum function length before flagging (default: 50)
        #[arg(long, default_value = "50")]
        max_lines: usize,

        /// Maximum parameter count before flagging (default: 4)
        #[arg(long, default_value = "4")]
        max_params: usize,

        /// Analyze only these specific files (repeatable).
        /// Useful for scripting: km smells --files src/foo.rs --files src/bar.ex
        #[arg(long, value_name = "FILE", conflicts_with = "since_ref")]
        files: Vec<PathBuf>,

        /// Analyze only files changed since this git ref (e.g. origin/main, HEAD~1).
        /// Runs git diff internally — no need to pipe file lists.
        /// Ideal for CI: km smells --since-ref origin/main
        #[arg(long, value_name = "REF")]
        since_ref: Option<String>,
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

        /// Compare current score against a git ref (default: HEAD).
        /// Shows how the score changed: "B- → B (+2.3)".
        /// Useful for PR review: --trend origin/main
        #[arg(long, num_args = 0..=1, default_missing_value = "HEAD", value_name = "REF")]
        trend: Option<String>,

        /// Exit with code 1 if the score is worse than the ref (requires --trend).
        /// Example: --trend origin/main --fail-if-worse
        #[arg(long, requires = "trend")]
        fail_if_worse: bool,

        /// Exit with code 1 if the score is below GRADE (requires --trend).
        /// Example: --trend origin/main --fail-below B-
        /// Valid grades: A++, A+, A, A-, B+, B, B-, C+, C, C-, D+, D, D-, F, F-, F--
        #[arg(long, value_name = "GRADE", requires = "trend")]
        fail_below: Option<String>,
    },

    /// Analyze code age: classify files as active, stale, or frozen by last git modification
    Age {
        #[command(flatten)]
        common: CommonArgs,

        /// Files modified within this many days are Active (default: 90)
        #[arg(long, default_value = "90")]
        active_days: u64,

        /// Files not modified for more than this many days are Frozen (default: 365)
        #[arg(long, default_value = "365")]
        frozen_days: u64,

        /// Sort by: date (oldest first, default), status, or file
        #[arg(long, default_value = "date", value_parser = ["date", "status", "file"])]
        sort_by: String,

        /// Show only files with this status: active, stale, or frozen
        #[arg(long, value_parser = ["active", "stale", "frozen"])]
        status: Option<String>,
    },

    /// Analyze internal module dependencies: fan-in, fan-out, and dependency cycles
    Deps {
        #[command(flatten)]
        common: CommonArgs,

        /// Show only files with dependency cycles
        #[arg(long)]
        cycles_only: bool,

        /// Sort by: fan-out (default), fan-in
        #[arg(long, default_value = "fan-out", value_parser = ["fan-out", "fan-in"])]
        sort_by: String,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,
    },

    /// Summarize code ownership by author: files owned, lines, languages, last active date
    Authors {
        #[command(flatten)]
        common: CommonArgs,

        /// Only consider activity since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,
    },

    /// AI-powered code analysis and tooling
    Ai {
        #[command(subcommand)]
        command: AiCommands,
    },

    /// Generate shell completion scripts (zsh, bash, fish, ...)
    Completions {
        /// Shell to generate completions for
        shell: Shell,
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

        /// Output format: table (default), json, short, or terse
        #[arg(long, value_enum, default_value_t)]
        format: OutputMode,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        #[command(flatten)]
        exclude_args: ExcludeArgs,

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

        /// Also configure permissions so km commands run without prompting
        #[arg(long)]
        with_permissions: bool,
    },

    /// Configure Claude Code permissions for km commands
    #[command(long_about = cli_help::AI_PERMISSIONS)]
    Permissions {
        /// Provider for permissions (e.g. claude)
        provider: String,
    },
}
