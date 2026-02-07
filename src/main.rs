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
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc { path } => {
            let target = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(err) = loc::run(&target) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
    }
}
