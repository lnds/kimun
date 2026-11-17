use clap::Subcommand;

use crate::options::CommonArgs;

/// Infrastructure subcommands.
#[derive(Subcommand)]
pub enum InfraCommands {
    /// Find security smells in infrastructure code: secrets, root, open
    /// addresses, plain HTTP, weak hashes, unpinned images and actions
    #[command(long_about = super::SMELLS_HELP)]
    Smells {
        #[command(flatten)]
        common: CommonArgs,

        /// Rows of the table (default: 20); JSON lists every file unless given
        #[arg(long)]
        top: Option<usize>,

        /// Only files changed since this git ref (e.g. origin/main)
        #[arg(long, value_name = "REF")]
        since_ref: Option<String>,

        /// Exit with code 1 when one of these smells is found (for CI);
        /// comma-separated
        #[arg(long, value_enum, value_delimiter = ',', value_name = "SMELL")]
        fail_on: Vec<super::Smell>,
    },
}
