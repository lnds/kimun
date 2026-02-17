/// `cm` — a CLI tool for comprehensive code metrics analysis.
///
/// Supports lines of code counting, duplicate detection, Halstead
/// complexity, cyclomatic complexity, indentation analysis,
/// Maintainability Index, hotspots, knowledge maps, temporal coupling,
/// and an overall code health score (A++ to F--).
mod ai;
mod cli;
mod cycom;
mod dups;
mod git;
mod hal;
mod hotspots;
mod indent;
mod knowledge;
mod loc;
mod mi;
mod miv;
mod report;
mod report_helpers;
mod score;
mod tc;
mod util;
mod walk;

use std::path::PathBuf;

use clap::Parser;

use cli::{AiCommands, Cli, Commands};

/// Resolve an optional path to a default of "." and run an analysis
/// command, printing errors to stderr and exiting with code 1 on failure.
fn run_command(
    path: Option<PathBuf>,
    f: impl FnOnce(&std::path::Path) -> Result<(), Box<dyn std::error::Error>>,
) {
    let target = path.unwrap_or_else(|| PathBuf::from("."));
    if let Err(err) = f(&target) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

/// Application entry point: parse CLI arguments and dispatch to the
/// appropriate analysis command. Each subcommand is delegated to its
/// corresponding module via `run_command`.
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc {
            path,
            verbose,
            json,
        } => run_command(path, |t| loc::run(t, verbose, json)),
        Commands::Dups {
            common,
            report,
            show_all,
            min_lines,
        } => run_command(common.path, |t| {
            dups::run(
                t,
                min_lines,
                report,
                show_all,
                common.json,
                !common.include_tests,
            )
        }),
        Commands::Indent { common } => run_command(common.path, |t| {
            indent::run(t, common.json, common.include_tests)
        }),
        Commands::Hal {
            common,
            top,
            sort_by,
        } => run_command(common.path, |t| {
            hal::run(t, common.json, common.include_tests, top, &sort_by)
        }),
        Commands::Cycom {
            common,
            min_complexity,
            top,
            per_function,
            sort_by,
        } => run_command(common.path, |t| {
            cycom::run(
                t,
                common.json,
                common.include_tests,
                min_complexity,
                top,
                per_function,
                &sort_by,
            )
        }),
        Commands::Mi {
            common,
            top,
            sort_by,
        } => run_command(common.path, |t| {
            mi::run(t, common.json, common.include_tests, top, &sort_by)
        }),
        Commands::Report {
            common,
            top,
            min_lines,
            full,
        } => {
            let effective_top = if full { usize::MAX } else { top };
            run_command(common.path, |t| {
                report::run(
                    t,
                    common.json,
                    common.include_tests,
                    effective_top,
                    min_lines,
                )
            });
        }
        Commands::Miv {
            common,
            top,
            sort_by,
        } => run_command(common.path, |t| {
            miv::run(t, common.json, common.include_tests, top, &sort_by)
        }),
        Commands::Hotspots {
            common,
            top,
            sort_by,
            since,
            complexity,
        } => run_command(common.path, |t| {
            hotspots::run(
                t,
                common.json,
                common.include_tests,
                top,
                &sort_by,
                since.as_deref(),
                &complexity,
            )
        }),
        Commands::Knowledge {
            common,
            top,
            sort_by,
            since,
            risk_only,
        } => run_command(common.path, |t| {
            knowledge::run(
                t,
                common.json,
                common.include_tests,
                top,
                &sort_by,
                since.as_deref(),
                risk_only,
            )
        }),
        Commands::Tc {
            common,
            top,
            sort_by,
            since,
            min_degree,
            min_strength,
        } => run_command(common.path, |t| {
            tc::run(
                t,
                common.json,
                common.include_tests,
                top,
                &sort_by,
                since.as_deref(),
                min_degree,
                min_strength,
            )
        }),
        Commands::Score {
            common,
            bottom,
            min_lines,
        } => run_command(common.path, |t| {
            score::run(t, common.json, common.include_tests, bottom, min_lines)
        }),
        Commands::Ai { command } => match command {
            AiCommands::Analyze {
                provider,
                path,
                model,
                output,
            } => run_command(path, |t| {
                ai::run(&provider, t, model.as_deref(), output.as_deref())
            }),
            AiCommands::Skill { provider } => {
                if let Err(err) = ai::skill::install(&provider) {
                    eprintln!("error: {err}");
                    std::process::exit(1);
                }
            }
        },
    }
}
