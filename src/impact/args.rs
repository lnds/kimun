use std::path::PathBuf;

use clap::Args;

use crate::options::OutputMode;

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

    /// Follow the structural radius at most N files away from the change
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    pub max_distance: Option<u32>,

    /// Print only the projects the diff changes or reaches, one per line (for CI)
    #[arg(long)]
    pub affected: bool,

    /// Exit with code 1 when a document that covers or names a changed file
    /// was not updated (for CI)
    #[arg(long)]
    pub fail_on_stale_docs: bool,

    /// Add the HTTP level: the routes the change removes or changes, and the
    /// client calls anywhere in the repository that ask for them (heuristic)
    #[arg(long)]
    pub http: bool,

    /// Show only the top N missing files (default: 20)
    #[arg(long, default_value = "20")]
    pub top: usize,
}
