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

fn parse_non_negative_f64(s: &str) -> Result<f64, String> {
    match s.parse::<f64>() {
        Ok(v) if v >= 0.0 => Ok(v),
        Ok(_) => Err("must not be negative".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// A share strictly between 0 and 100: at 100 no departure could ever
/// orphan more than all of the code.
fn parse_bus_factor_threshold(s: &str) -> Result<f64, String> {
    match s.parse::<f64>() {
        Ok(v) if v > 0.0 && v < 100.0 => Ok(v),
        Ok(_) => Err("must be greater than 0 and less than 100".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

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

/// What `km score --fail-if-worse` compares against the ref.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, ValueEnum)]
pub enum GateScope {
    /// The project's aggregate score
    #[default]
    Project,
    /// Modified files that end below the project score, plus duplicated lines
    Changed,
}

impl GateScope {
    /// A single file's score moves far more per edited line than the project's.
    pub fn default_tolerance(self) -> f64 {
        match self {
            Self::Project => 0.01,
            Self::Changed => 0.5,
        }
    }
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
}

/// Common arguments shared by most analysis commands.
#[derive(Args)]
pub struct CommonArgs {
    /// Directory to analyze (default: current directory)
    pub path: Option<PathBuf>,

    /// Output format: table (default), json, short, terse, github, or codeclimate.
    /// `short` emits a single compact line of key:value pairs (AI-friendly).
    /// `terse` emits a single headline metric value (for piping/embedding).
    /// `github` emits GitHub Actions warning annotations (only cycom, cogcom, smells).
    /// `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality (only cycom, cogcom, smells).
    #[arg(long, value_enum, default_value_t)]
    pub format: OutputMode,

    /// Include tests, examples and benchmarks in analysis (excluded by default)
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

/// All available analysis subcommands.
/// Arguments of `km impact`. One source of the change is required; a patch
/// may come with `--since-ref`, which then only bounds the history.
#[derive(Args)]
pub struct ImpactArgs {
    /// Where to look for the repository (default: current directory).
    /// The whole repository is measured, not only this directory.
    pub path: Option<PathBuf>,

    /// Output format: table (default), json, short, or terse
    #[arg(long, value_enum, default_value_t)]
    pub format: OutputMode,

    /// Measure the diff between this git ref and the working tree (e.g. origin/main).
    /// With --diff, it only bounds the history consulted.
    #[arg(
        long,
        value_name = "REF",
        required_unless_present_any = ["diff", "pr"],
        conflicts_with = "pr"
    )]
    pub since_ref: Option<String>,

    /// Measure up to this ref instead of the working tree: what it brings
    /// since it diverged from --since-ref, whatever is checked out
    #[arg(long, value_name = "REF", requires = "since_ref", conflicts_with_all = ["diff", "pr"])]
    pub until_ref: Option<String>,

    /// Measure a patch in git format, from a file or from stdin (`-`)
    #[arg(long, value_name = "FILE", conflicts_with = "pr")]
    pub diff: Option<PathBuf>,

    /// Measure a GitHub pull request by number. Requires the GitHub CLI
    /// (`gh`) installed and authenticated
    #[arg(long, value_name = "NUMBER")]
    pub pr: Option<u64>,

    /// Only consider history since this time (e.g. 6m, 1y, 30d)
    #[arg(long)]
    pub since: Option<String>,

    /// Minimum share of a changed file's commits that also changed the missing file
    #[arg(long, default_value = "0.5")]
    pub min_confidence: f64,

    /// Minimum commits a changed file and a missing file must share
    #[arg(long, default_value = "3")]
    pub min_shared: usize,

    /// Ignore commits touching more than N files as evidence of co-change
    #[arg(long, default_value = "30", value_name = "N")]
    pub max_changeset: usize,

    /// Print only the projects the diff changes or reaches, one per line (for CI)
    #[arg(long)]
    pub affected: bool,

    /// Show only the top N missing files (default: 20)
    #[arg(long, default_value = "20")]
    pub top: usize,
}

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

        /// Minimum lines for a duplicate block (default: 6, overridable via .kimun.toml)
        #[arg(long)]
        min_lines: Option<usize>,

        /// Exit with code 1 if duplicate groups exceed this limit.
        /// Useful as a CI quality gate: --max-duplicates 0 fails on any duplicate.
        /// Overridable via .kimun.toml [dups] max_duplicates.
        #[arg(long, value_name = "N")]
        max_duplicates: Option<usize>,

        /// Exit with code 1 if the duplicated-lines ratio exceeds this percentage.
        /// Useful for ratcheting down duplication over time: --max-dup-ratio 5.0
        /// fails when more than 5% of code lines are duplicated.
        /// Overridable via .kimun.toml [dups] max_dup_ratio.
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

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

        /// Sort by metric: effort, volume, or bugs (default: effort)
        #[arg(long, default_value = "effort", value_parser = ["effort", "volume", "bugs"])]
        sort_by: String,
    },

    /// Analyze cyclomatic complexity per file and per function
    Cycom {
        #[command(flatten)]
        common: CommonArgs,

        /// Skip files whose most complex function is below N; with --per-function, also hide functions below N (default: 1)
        #[arg(long, default_value = "1")]
        min_complexity: usize,

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

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

        /// Skip files whose most complex function is below N; with --per-function, also hide functions below N (default: 1)
        #[arg(long, default_value = "1")]
        min_complexity: usize,

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

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

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

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

        /// Minimum lines for a duplicate block (default: 6, overridable via .kimun.toml)
        #[arg(long)]
        min_lines: Option<usize>,

        /// Show all files instead of truncating to top N
        #[arg(long)]
        full: bool,
    },

    /// Compute Maintainability Index per file (verifysoft variant, with comment weight)
    #[command(long_about = cli_help::MIV)]
    Miv {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Analyze code churn: pure change frequency per file (git commits only)
    Churn {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

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

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

        /// Sort by metric: score, commits, or complexity (default: score)
        #[arg(long, default_value = "score", value_parser = ["score", "commits", "complexity"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Complexity metric: indent (default, Thornhill), cycom (cyclomatic), or cogcom (cognitive).
        /// Overridable via .kimun.toml [hotspots] complexity.
        #[arg(long, value_parser = ["indent", "cycom", "cogcom"])]
        complexity: Option<String>,
    },

    /// Analyze code ownership patterns via git blame (knowledge maps)
    #[command(long_about = cli_help::KNOWLEDGE)]
    Knowledge {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

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

        /// Compute the project bus factor: how many contributors have to leave
        /// before most of the code is in files with no author left. With
        /// --since, authors inactive in that window have already left.
        #[arg(long)]
        bus_factor: bool,

        /// Share of the code, in lines, the bus factor counts to (default: 50)
        #[arg(long, value_name = "PCT", default_value = "50", value_parser = parse_bus_factor_threshold)]
        bus_factor_threshold: f64,

        /// Show only files owned by this author (substring match, case-insensitive)
        #[arg(long, value_name = "NAME")]
        author: Option<String>,
    },

    /// Analyze temporal coupling: files that change together in commits
    #[command(long_about = cli_help::TC)]
    Tc {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every one of the file pairs unless given
        #[arg(long)]
        top: Option<usize>,

        /// Sort by: strength or shared (default: strength)
        #[arg(long, default_value = "strength", value_parser = ["strength", "shared"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Minimum commits per file to be included (default: 3, overridable via .kimun.toml)
        #[arg(long)]
        min_degree: Option<usize>,

        /// Filter results: show only pairs with strength >= threshold (e.g. 0.5 for strong coupling only).
        /// Overridable via .kimun.toml [tc] min_strength.
        #[arg(long)]
        min_strength: Option<f64>,

        /// Minimum commits two files must share to be a pair
        #[arg(long, default_value = "3")]
        min_shared: usize,

        /// Ignore commits touching more than N files as evidence of coupling
        #[arg(long, default_value = "30", value_name = "N")]
        max_changeset: usize,
    },

    /// Measure the impact of a diff: how spread it is and which files usually change with it
    #[command(long_about = crate::impact::HELP)]
    Impact(ImpactArgs),

    /// Detect common code smells per file
    #[command(long_about = cli_help::SMELLS)]
    Smells {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every one of the files unless given
        #[arg(long)]
        top: Option<usize>,

        /// Maximum function length before flagging (default: 50, overridable via .kimun.toml)
        #[arg(long)]
        max_lines: Option<usize>,

        /// Maximum parameter count before flagging (default: 4, overridable via .kimun.toml)
        #[arg(long)]
        max_params: Option<usize>,

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

        /// Minimum lines for a duplicate block (default: 6, overridable via .kimun.toml)
        #[arg(long)]
        min_lines: Option<usize>,

        /// Scoring model: cogcom (default), cogcom-0.35 (the weights up to v0.35)
        /// or legacy (MI + cyclomatic, v0.13).
        /// Overridable via .kimun.toml [score] model.
        #[arg(long, value_parser = ["cogcom", "cogcom-0.35", "legacy"])]
        model: Option<String>,

        /// Score with the experimental model: the Maintainability Index
        /// (Visual Studio variant) in place of Halstead Effort, shown against
        /// the score of the default model.
        #[arg(long, conflicts_with_all = ["model", "trend"])]
        experimental: bool,

        /// Compare current score against a git ref (default: HEAD).
        /// Shows how the score changed: "B- → B (+2.3)".
        /// Useful for PR review: --trend origin/main
        #[arg(long, num_args = 0..=1, default_missing_value = "HEAD", value_name = "REF")]
        trend: Option<String>,

        /// Exit with code 1 if the score is worse than the ref (requires --trend).
        /// Example: --trend origin/main --fail-if-worse
        #[arg(long, requires = "trend")]
        fail_if_worse: bool,

        /// Score drop allowed by --fail-if-worse before it fails
        /// (default: 0.01 with --gate-scope project, 0.5 with --gate-scope changed).
        /// Example: --fail-if-worse --gate-tolerance 0.05
        #[arg(long, value_name = "POINTS",
              value_parser = parse_non_negative_f64, requires = "fail_if_worse")]
        gate_tolerance: Option<f64>,

        /// What --fail-if-worse compares: `project` (aggregate score) or `changed`
        /// (modified files already below the project score, plus duplicated lines;
        /// deleting code or touching healthy files cannot fail it)
        #[arg(long, value_enum, default_value_t = GateScope::Project, requires = "fail_if_worse")]
        gate_scope: GateScope,

        /// Exit with code 1 if the score is below GRADE (requires --trend).
        /// Example: --trend origin/main --fail-below B-
        /// Valid grades: A++, A+, A, A-, B+, B, B-, C+, C, C-, D+, D, D-, F, F-, F--
        /// Overridable via .kimun.toml [score] fail_below.
        #[arg(long, value_name = "GRADE", requires = "trend")]
        fail_below: Option<String>,
    },

    /// Analyze code age: classify files as active, stale, or frozen by last git modification
    Age {
        #[command(flatten)]
        common: CommonArgs,

        /// Files modified within this many days are Active (default: 90, overridable via .kimun.toml)
        #[arg(long)]
        active_days: Option<u64>,

        /// Files not modified for more than this many days are Frozen (default: 365, overridable via .kimun.toml)
        #[arg(long)]
        frozen_days: Option<u64>,

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

        #[command(flatten)]
        args: DepsArgs,
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

    /// Analyze the project and generate a calibrated .kimun.toml
    Init {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Write .kimun.toml without prompting
        #[arg(short, long)]
        yes: bool,
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

        /// Include tests, examples and benchmarks in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        #[command(flatten)]
        exclude_args: ExcludeArgs,

        /// Number of worst files to show in "needs attention" (default: 10)
        #[arg(long, default_value = "10")]
        bottom: usize,

        /// Minimum lines for a duplicate block (default: 6, overridable via .kimun.toml)
        #[arg(long)]
        min_lines: Option<usize>,

        /// Scoring model: cogcom (default), cogcom-0.35 (the weights up to v0.35)
        /// or legacy (MI + cyclomatic, v0.13).
        /// Overridable via .kimun.toml [score] model.
        #[arg(long, value_parser = ["cogcom", "cogcom-0.35", "legacy"])]
        model: Option<String>,
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
