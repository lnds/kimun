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

/// Code age analysis: Active / Stale / Frozen classification by last git modification.
mod age;
/// AI-powered analysis via external LLM providers.
mod ai;
/// Author summary: per-author ownership, lines, languages, last active date.
mod authors;
/// Code churn analysis: pure change frequency per file from git history.
mod churn;
/// CLI argument definitions using `clap` derive macros.
mod cli;
/// Long help text constants extracted from CLI definitions.
mod cli_help;
/// Cognitive complexity analysis (SonarSource, 2017).
mod cogcom;
/// Cyclomatic complexity analysis (per-file and per-function).
mod cycom;
/// Dependency graph analysis: internal module coupling via import parsing.
mod deps;
/// Shared function detection for complexity analyzers.
mod detection;
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
/// Code smell detection (long functions, magic numbers, etc.).
mod smells;
/// Temporal coupling analysis (co-changing files in git history).
mod tc;
/// Shared utilities (string masking, file reading, since parsing).
mod util;
/// Filesystem walking with .gitignore support and test exclusion.
mod walk;

use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};

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

/// Build filter, handle `--list-excluded`, then run a walk-based command.
///
/// Exposes `$json` (bool) and `$cfg` (`WalkConfig`) inside `$body`.
/// Saves ~7 lines of identical boilerplate per subcommand arm.
///
/// The `$json` identifier is explicit in the call site pattern so that Rust's
/// macro hygiene allows it to be referenced inside `$body`.
macro_rules! dispatch {
    ($common:expr, |$cfg:ident, $json:ident| $body:expr) => {{
        let _c = $common;
        let include_tests = _c.include_tests;
        let $json = _c.json;
        let filter = _c.exclude_filter();
        maybe_list_excluded(&_c.path, include_tests, &filter, _c.list_excluded());
        run_command(_c.path, |t| {
            let $cfg = WalkConfig::new(t, include_tests, &filter);
            $body
        })
    }};
}

/// Dispatch the `tc` subcommand. Temporal coupling works entirely from git
/// history, so filesystem exclude flags have no effect — warn the user.
fn dispatch_tc(
    common: cli::CommonArgs,
    top: usize,
    sort_by: String,
    since: Option<String>,
    min_degree: usize,
    min_strength: Option<f64>,
) {
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
    });
}

/// Dispatch the `smells` subcommand. Supports three modes: `--since-ref`,
/// explicit `--files`, or a full directory walk.
fn dispatch_smells(
    common: cli::CommonArgs,
    top: usize,
    max_lines: usize,
    max_params: usize,
    files: Vec<PathBuf>,
    since_ref: Option<String>,
    format: Option<String>,
) {
    let include_tests = common.include_tests;
    let json = common.json;
    let filter = common.exclude_filter();
    maybe_list_excluded(&common.path, include_tests, &filter, common.list_excluded());
    run_command(common.path, |t| {
        if let Some(ref git_ref) = since_ref {
            let git_repo =
                git::GitRepo::open(t).map_err(|e| format!("not a git repository: {e}"))?;
            let changed = git_repo.files_changed_since(git_ref)?;
            smells::run_on_files(
                &changed,
                json,
                top,
                max_lines,
                max_params,
                format.as_deref(),
            )
        } else if !files.is_empty() {
            smells::run_on_files(&files, json, top, max_lines, max_params, format.as_deref())
        } else {
            let cfg = WalkConfig::new(t, include_tests, &filter);
            smells::run(&cfg, json, top, max_lines, max_params, format.as_deref())
        }
    });
}

/// Dispatch `score` (no subcommand). Parses `--fail-below` early so errors
/// surface before analysis runs, then delegates to `run_diff` or `run`.
fn dispatch_score(
    common: cli::CommonArgs,
    bottom: usize,
    min_lines: usize,
    model: String,
    trend: Option<String>,
    fail_if_worse: bool,
    fail_below: Option<String>,
) {
    let fail_below_grade = match fail_below {
        Some(ref s) => match score::analyzer::Grade::parse(s) {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("error: --fail-below: {e}");
                std::process::exit(1);
            }
        },
        None => None,
    };
    dispatch!(common, |cfg, json| {
        if let Some(ref git_ref) = trend {
            let gate = score::ScoreGate {
                fail_if_worse,
                fail_below: fail_below_grade,
            };
            score::run_diff(&cfg, git_ref, json, bottom, min_lines, &model, gate)
        } else {
            score::run(&cfg, json, bottom, min_lines, &model)
        }
    });
}

/// Dispatch the `ai` subcommand and its nested commands.
fn dispatch_ai(command: AiCommands) {
    match command {
        AiCommands::Analyze {
            provider,
            path,
            model,
            output,
        } => run_command(path, |t| {
            ai::run(&provider, t, model.as_deref(), output.as_deref())
        }),
        AiCommands::Skill {
            provider,
            with_permissions,
        } => {
            if let Err(err) = ai::skill::install(&provider, with_permissions) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        AiCommands::Permissions { provider } => {
            if provider != "claude" {
                eprintln!("error: Unsupported provider: {provider}. Supported: claude");
                std::process::exit(1);
            }
            let repo = git2::Repository::discover(".").expect("Could not find a git repository");
            let workdir = repo
                .workdir()
                .expect("Could not determine repository working directory");
            if let Err(err) = ai::permissions::install(workdir) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
    }
}

/// Application entry point: parse CLI arguments and dispatch to the
/// appropriate analysis command. Each subcommand is delegated to its
/// corresponding module via `run_command`.
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc {
            common,
            verbose,
            by_author,
        } => dispatch!(common, |cfg, json| {
            if by_author {
                loc::run_by_author(&cfg, json)
            } else {
                loc::run(&cfg, verbose, json)
            }
        }),
        Commands::Dups {
            common,
            report,
            show_all,
            min_lines,
            max_duplicates,
            max_dup_ratio,
            fail_on_increase,
        } => {
            dispatch!(common, |cfg, json| {
                let gate = dups::DupsGate {
                    max_duplicates,
                    max_dup_ratio,
                    fail_on_increase: fail_on_increase.clone(),
                };
                dups::run(&cfg, min_lines, report, show_all, json, gate)
            })
        }
        Commands::Indent { common } => dispatch!(common, |cfg, json| indent::run(&cfg, json)),
        Commands::Hal {
            common,
            top,
            sort_by,
        } => {
            dispatch!(common, |cfg, json| hal::run(&cfg, json, top, &sort_by))
        }
        Commands::Cycom {
            common,
            min_complexity,
            top,
            per_function,
            sort_by,
            format,
        } => {
            dispatch!(common, |cfg, json| {
                cycom::run(
                    &cfg,
                    json,
                    min_complexity,
                    top,
                    per_function,
                    &sort_by,
                    format.as_deref(),
                )
            })
        }
        Commands::Cogcom {
            common,
            min_complexity,
            top,
            per_function,
            sort_by,
            format,
        } => {
            dispatch!(common, |cfg, json| {
                cogcom::run(
                    &cfg,
                    json,
                    min_complexity,
                    top,
                    per_function,
                    &sort_by,
                    format.as_deref(),
                )
            })
        }
        Commands::Mi {
            common,
            top,
            sort_by,
        } => {
            dispatch!(common, |cfg, json| mi::run(&cfg, json, top, &sort_by))
        }
        Commands::Report {
            common,
            top,
            min_lines,
            full,
        } => {
            let effective_top = if full { usize::MAX } else { top };
            dispatch!(common, |cfg, json| report::run(
                &cfg,
                json,
                effective_top,
                min_lines
            ))
        }
        Commands::Miv {
            common,
            top,
            sort_by,
        } => {
            dispatch!(common, |cfg, json| miv::run(&cfg, json, top, &sort_by))
        }
        Commands::Churn {
            common,
            top,
            sort_by,
            since,
        } => {
            dispatch!(common, |cfg, json| churn::run(
                &cfg,
                json,
                top,
                &sort_by,
                since.as_deref()
            ))
        }
        Commands::Hotspots {
            common,
            top,
            sort_by,
            since,
            complexity,
        } => {
            dispatch!(common, |cfg, json| {
                hotspots::run(&cfg, json, top, &sort_by, since.as_deref(), &complexity)
            })
        }
        Commands::Age {
            common,
            active_days,
            frozen_days,
            sort_by,
            status,
        } => {
            dispatch!(common, |cfg, json| {
                age::run(
                    &cfg,
                    json,
                    active_days,
                    frozen_days,
                    &sort_by,
                    status.as_deref(),
                )
            })
        }
        Commands::Knowledge {
            common,
            top,
            sort_by,
            since,
            risk_only,
            summary,
            bus_factor,
            author,
        } => {
            dispatch!(common, |cfg, json| {
                knowledge::run(
                    &cfg,
                    &knowledge::KnowledgeOptions {
                        json,
                        top,
                        sort_by: &sort_by,
                        since: since.as_deref(),
                        risk_only,
                        summary,
                        bus_factor,
                        author: author.as_deref(),
                    },
                )
            })
        }
        Commands::Deps {
            common,
            cycles_only,
            sort_by,
            top,
        } => {
            dispatch!(common, |cfg, json| deps::run(
                &cfg,
                json,
                cycles_only,
                &sort_by,
                top
            ))
        }
        Commands::Authors { common, since } => {
            dispatch!(common, |cfg, json| authors::run(
                &cfg,
                json,
                since.as_deref()
            ))
        }
        Commands::Tc {
            common,
            top,
            sort_by,
            since,
            min_degree,
            min_strength,
        } => dispatch_tc(common, top, sort_by, since, min_degree, min_strength),
        Commands::Smells {
            common,
            top,
            max_lines,
            max_params,
            files,
            since_ref,
            format,
        } => dispatch_smells(common, top, max_lines, max_params, files, since_ref, format),
        Commands::Score {
            subcommand: None,
            common,
            bottom,
            min_lines,
            model,
            trend,
            fail_if_worse,
            fail_below,
        } => dispatch_score(
            common,
            bottom,
            min_lines,
            model,
            trend,
            fail_if_worse,
            fail_below,
        ),
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
                score::run_diff(
                    &cfg,
                    &git_ref,
                    json,
                    bottom,
                    min_lines,
                    &model,
                    score::ScoreGate::default(),
                )
            })
        }
        Commands::Ai { command } => dispatch_ai(command),
        Commands::Completions { shell } => {
            write_completions(shell, &mut std::io::stdout());
        }
    }
}

/// Generate shell completions for `km` into `buf`.
pub fn write_completions(shell: Shell, buf: &mut impl std::io::Write) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "km", buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_zsh_starts_with_compdef() {
        let mut buf = Vec::new();
        write_completions(Shell::Zsh, &mut buf);
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.starts_with("#compdef km"),
            "zsh script should start with #compdef km"
        );
    }

    #[test]
    fn completions_bash_contains_km() {
        let mut buf = Vec::new();
        write_completions(Shell::Bash, &mut buf);
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("km"), "bash completion should reference km");
    }

    #[test]
    fn completions_fish_contains_km() {
        let mut buf = Vec::new();
        write_completions(Shell::Fish, &mut buf);
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("km"), "fish completion should reference km");
    }

    #[test]
    fn completions_zsh_includes_subcommands() {
        let mut buf = Vec::new();
        write_completions(Shell::Zsh, &mut buf);
        let out = String::from_utf8(buf).unwrap();
        for cmd in [
            "loc",
            "dups",
            "score",
            "knowledge",
            "hotspots",
            "smells",
            "completions",
        ] {
            assert!(
                out.contains(cmd),
                "zsh completion missing subcommand: {cmd}"
            );
        }
    }
}
