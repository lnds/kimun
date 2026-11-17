use clap::Subcommand;

use crate::options::CommonArgs;

/// Documentation subcommands.
#[derive(Subcommand)]
pub enum DocsCommands {
    /// Find references in Markdown documents that lead nowhere: links and
    /// images to missing files or headings, paths in code spans
    #[command(long_about = super::REFS_HELP)]
    Refs {
        #[command(flatten)]
        common: CommonArgs,

        /// Exit with code 1 when a dead reference is found (for CI)
        #[arg(long)]
        fail_on_dead: bool,

        /// Only documents changed since this git ref, and references to
        /// files changed since it (e.g. origin/main)
        #[arg(long, value_name = "REF")]
        since_ref: Option<String>,
    },

    /// The context files of coding agents (CLAUDE.md, AGENTS.md, rules,
    /// skills): tokens per file and per load, rules, repetition, change
    #[command(long_about = super::CONTEXT_HELP)]
    Context {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of each table (default: 20); JSON lists every entry
        #[arg(long)]
        top: Option<usize>,

        /// Exit with code 1 when what an agent reads in a directory goes
        /// over N tokens (for CI)
        #[arg(long, value_name = "N")]
        max_tokens: Option<usize>,
    },

    /// The links between documents as a graph: broken links, orphan
    /// notes, hubs and islands
    #[command(long_about = super::LINKS_HELP)]
    Links {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of each table (default: 20); JSON lists every entry unless given
        #[arg(long)]
        top: Option<usize>,
    },

    /// Find documents behind the code they describe: files a document
    /// covers or names changed after it
    #[command(long_about = super::STALE_HELP)]
    Stale {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every document unless given
        #[arg(long)]
        top: Option<usize>,
    },
}
