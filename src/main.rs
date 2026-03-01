//! `km` (Kimün) — a CLI tool for comprehensive code metrics analysis.
//!
//! Supports 12 analysis commands covering static metrics (LOC, duplication,
//! Halstead, cyclomatic, indentation, MI, code health score) and git-based
//! metrics (hotspots, knowledge maps, temporal coupling). Each command is
//! a standalone module that handles its own analysis and reporting.
//!
//! The dispatch pattern is uniform: parse CLI args with `clap`, resolve
//! the target path (defaulting to "."), and delegate to the module's `run()`
//! function. All errors are printed to stderr and cause exit code 1.

/// AI-powered analysis via external LLM providers.
mod ai;
/// CLI argument definitions using `clap` derive macros.
mod cli;
/// Long help text constants extracted from CLI definitions.
mod cli_help;
/// Cognitive complexity analysis (SonarSource, 2017).
mod cogcom;
/// Cyclomatic complexity analysis (per-file and per-function).
mod cycom;
/// Duplicate code detection using sliding-window fingerprinting.
mod dups;
/// Git repository access via libgit2 (change frequency, blame, coupling).
mod git;
/// Halstead complexity metrics (volume, effort, bugs, time).
mod hal;
/// Hotspot analysis: change frequency × complexity.
mod hotspots;
/// Indentation complexity (stddev and max depth).
mod indent;
/// Knowledge maps: code ownership via git blame.
mod knowledge;
/// Lines of code counting with FSM-based line classification.
mod loc;
/// Maintainability Index (Visual Studio variant, 0–100 scale).
mod mi;
/// Maintainability Index (verifysoft variant, with comment weight).
mod miv;
/// Combined report (`km report`) aggregating all metrics.
mod report;
/// Shared report formatting utilities (separators, path widths, JSON output).
mod report_helpers;
/// Overall code health score (A++ to F--, 5 weighted dimensions).
mod score;
/// Temporal coupling analysis (co-changing files in git history).
mod tc;
/// Shared utilities (string masking, file reading, since parsing).
mod util;
/// Filesystem walking with .gitignore support and test exclusion.
mod walk;

use std::path::PathBuf;

use clap::Parser;

use cli::{AiCommands, Cli, Commands, ScoreCommands};

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
        Commands::Loc { common, verbose } => run_command(common.path, |t| {
            loc::run(t, verbose, common.json, common.include_tests)
        }),
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
        Commands::Cogcom {
            common,
            min_complexity,
            top,
            per_function,
            sort_by,
        } => run_command(common.path, |t| {
            cogcom::run(
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
            subcommand: None,
            common,
            bottom,
            min_lines,
            model,
        } => run_command(common.path, |t| {
            score::run(
                t,
                common.json,
                common.include_tests,
                bottom,
                min_lines,
                &model,
            )
        }),
        Commands::Score {
            subcommand:
                Some(ScoreCommands::Diff {
                    git_ref,
                    path,
                    json,
                    include_tests,
                    bottom,
                    min_lines,
                    model,
                }),
            ..
        } => run_command(path, |t| {
            score::run_diff(t, &git_ref, json, include_tests, bottom, min_lines, &model)
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
