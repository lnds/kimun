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
use walk::{ExcludeFilter, WalkConfig};

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

/// If `--list-excluded` was passed, print the excluded files and exit.
fn maybe_list_excluded(
    path: &Option<PathBuf>,
    include_tests: bool,
    filter: &ExcludeFilter,
    list_excluded: bool,
) {
    if !list_excluded {
        return;
    }
    let target = path.as_deref().unwrap_or(std::path::Path::new("."));
    if let Err(err) = walk::print_excluded_files(target, !include_tests, filter) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
    std::process::exit(0);
}

/// Application entry point: parse CLI arguments and dispatch to the
/// appropriate analysis command. Each subcommand is delegated to its
/// corresponding module via `run_command`.
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc { common, verbose } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                loc::run(&cfg, verbose, common.json)
            })
        }
        Commands::Dups {
            common,
            report,
            show_all,
            min_lines,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                dups::run(&cfg, min_lines, report, show_all, common.json)
            })
        }
        Commands::Indent { common } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                indent::run(&cfg, common.json)
            })
        }
        Commands::Hal {
            common,
            top,
            sort_by,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                hal::run(&cfg, common.json, top, &sort_by)
            })
        }
        Commands::Cycom {
            common,
            min_complexity,
            top,
            per_function,
            sort_by,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                cycom::run(
                    &cfg,
                    common.json,
                    min_complexity,
                    top,
                    per_function,
                    &sort_by,
                )
            })
        }
        Commands::Cogcom {
            common,
            min_complexity,
            top,
            per_function,
            sort_by,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                cogcom::run(
                    &cfg,
                    common.json,
                    min_complexity,
                    top,
                    per_function,
                    &sort_by,
                )
            })
        }
        Commands::Mi {
            common,
            top,
            sort_by,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                mi::run(&cfg, common.json, top, &sort_by)
            })
        }
        Commands::Report {
            common,
            top,
            min_lines,
            full,
        } => {
            let effective_top = if full { usize::MAX } else { top };
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                report::run(&cfg, common.json, effective_top, min_lines)
            });
        }
        Commands::Miv {
            common,
            top,
            sort_by,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                miv::run(&cfg, common.json, top, &sort_by)
            })
        }
        Commands::Hotspots {
            common,
            top,
            sort_by,
            since,
            complexity,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                hotspots::run(
                    &cfg,
                    common.json,
                    top,
                    &sort_by,
                    since.as_deref(),
                    &complexity,
                )
            })
        }
        Commands::Knowledge {
            common,
            top,
            sort_by,
            since,
            risk_only,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                knowledge::run(
                    &cfg,
                    common.json,
                    top,
                    &sort_by,
                    since.as_deref(),
                    risk_only,
                )
            })
        }
        Commands::Tc {
            common,
            top,
            sort_by,
            since,
            min_degree,
            min_strength,
        } => {
            // tc works entirely from git commit data, not the filesystem,
            // so --exclude-ext/--exclude-dir/--exclude have no effect.
            if !common.exclude_args.is_empty() {
                eprintln!(
                    "warning: --exclude-ext/--exclude-dir/--exclude have no effect on `tc` \
                     (temporal coupling works from git history, not the filesystem)"
                );
            }
            run_command(common.path, |t| {
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
            })
        }
        Commands::Score {
            subcommand: None,
            common,
            bottom,
            min_lines,
            model,
        } => {
            let filter = common.exclude_filter();
            maybe_list_excluded(
                &common.path,
                common.include_tests,
                &filter,
                common.list_excluded(),
            );
            run_command(common.path, |t| {
                let cfg = WalkConfig::new(t, common.include_tests, &filter);
                score::run(&cfg, common.json, bottom, min_lines, &model)
            })
        }
        Commands::Score {
            subcommand:
                Some(ScoreCommands::Diff {
                    git_ref,
                    path,
                    json,
                    include_tests,
                    exclude_args,
                    bottom,
                    min_lines,
                    model,
                }),
            ..
        } => {
            let filter = exclude_args.exclude_filter();
            maybe_list_excluded(&path, include_tests, &filter, exclude_args.list_excluded);
            run_command(path, |t| {
                let cfg = WalkConfig::new(t, include_tests, &filter);
                score::run_diff(&cfg, &git_ref, json, bottom, min_lines, &model)
            })
        }
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
