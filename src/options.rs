//! The options every command shares: the output format, the files read
//! and left out, and what `--fail-if-worse` compares.

use std::path::PathBuf;

use clap::{Args, ValueEnum};

use crate::kind::{self, Kind, Selection};
use crate::walk::ExcludeFilter;

/// Error returned by commands that do not support CI annotation formats.
pub const ERR_CI_FORMAT_ONLY: &str = "--format github and --format codeclimate are only supported by cycom, cogcom, smells, impact, docs refs and infra smells";

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

    /// Read only files of these kinds: code, docs, infra, data or all.
    /// Without it each command reads what it always has.
    /// Repeatable or comma-separated: --kind code,infra
    #[arg(long, value_enum, value_delimiter = ',', value_name = "KIND")]
    pub kind: Vec<KindArg>,

    /// Print files that would be excluded by the current filter and exit.
    /// Useful for debugging filter rules before running an analysis.
    #[arg(long)]
    pub list_excluded: bool,
}

impl ExcludeArgs {
    /// Build an `ExcludeFilter` from the parsed CLI flags and the
    /// `[exclude]` and `[kinds]` sections of `.kimun.toml`.
    pub fn exclude_filter(&self) -> ExcludeFilter {
        let config = crate::config::KimunConfig::load();
        self.exclude_filter_with(&config.exclude)
            .with_kinds(self.kinds(&config.kinds))
    }

    /// The filter of the flags plus what `config` excludes: both apply.
    pub fn exclude_filter_with(&self, config: &crate::config::ExcludeConfig) -> ExcludeFilter {
        let joined = |cli: &[String], cfg: &[String]| [cli, cfg].concat();
        ExcludeFilter::new(
            &self.include_ext,
            &joined(&self.exclude_ext, &config.extensions),
            &joined(&self.exclude_dir, &config.dirs),
            &joined(&self.exclude, &config.patterns),
        )
    }

    /// The kinds of `--kind`, with what `config` declares.
    pub fn kinds(&self, config: &crate::config::KindsConfig) -> Selection {
        let chosen: Vec<Kind> = self
            .kind
            .iter()
            .flat_map(|arg| match arg {
                KindArg::Code => vec![Kind::Code],
                KindArg::Docs => vec![Kind::Docs],
                KindArg::Infra => vec![Kind::Infra],
                KindArg::Data => vec![Kind::Data],
                KindArg::All => kind::ALL.to_vec(),
            })
            .collect();
        Selection::new(&chosen, config)
    }
}

/// A value of `--kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum KindArg {
    Code,
    Docs,
    Infra,
    Data,
    All,
}

/// Common arguments shared by most analysis commands.
#[derive(Args)]
pub struct CommonArgs {
    /// Directory to analyze (default: current directory)
    pub path: Option<PathBuf>,

    /// Output format: table (default), json, short, terse, github, or codeclimate.
    /// `short` emits a single compact line of key:value pairs (AI-friendly).
    /// `terse` emits a single headline metric value (for piping/embedding).
    /// `github` emits GitHub Actions warning annotations (only cycom, cogcom, smells, impact, docs refs).
    /// `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality (only cycom, cogcom, smells, impact, docs refs).
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
