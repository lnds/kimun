mod ai;
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

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cm", version, about = "Code metrics tools")]
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

        /// Show summary stats (files read, unique, ignored, elapsed time)
        #[arg(short, long)]
        verbose: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Detect duplicate code across files
    Dups {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Show detailed report with duplicate locations
        #[arg(short, long)]
        report: bool,

        /// Show all duplicate groups (default: top 20)
        #[arg(long)]
        show_all: bool,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,
    },

    /// Analyze indentation complexity (stddev and max depth per file)
    Indent {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,
    },

    /// Analyze Halstead complexity metrics per file
    #[command(long_about = "\
Analyze Halstead complexity metrics per file.

Halstead metrics measure software complexity based on operators and operands
extracted from source code.

Base counts:
  n1 = distinct operators    n2 = distinct operands
  N1 = total operators       N2 = total operands

Derived metrics:
  Vocabulary (n) = n1 + n2
  Length (N)     = N1 + N2
  Volume (V)     = N * log2(n)       -- size of the implementation
  Difficulty (D) = (n1/2) * (N2/n2)  -- error proneness
  Effort (E)     = D * V             -- mental effort to develop
  Bugs (B)       = V / 3000          -- estimated delivered bugs
  Time (T)       = E / 18 seconds    -- estimated development time

Higher effort/volume/bugs indicate more complex and error-prone code.")]
    Hal {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: effort, volume, or bugs (default: effort)
        #[arg(long, default_value = "effort", value_parser = ["effort", "volume", "bugs"])]
        sort_by: String,
    },

    /// Analyze cyclomatic complexity per file and per function
    Cycom {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Minimum max-complexity to include a file (default: 1)
        #[arg(long, default_value = "1")]
        min_complexity: usize,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Show per-function breakdown
        #[arg(long)]
        per_function: bool,

        /// Sort by metric: total, max, or avg (default: total)
        #[arg(long, default_value = "total", value_parser = ["total", "max", "avg"])]
        sort_by: String,
    },

    /// Compute Maintainability Index per file (Visual Studio variant, 0-100 scale)
    #[command(long_about = "\
Compute Maintainability Index (MI) per file using the Visual Studio variant.

This variant normalizes MI to a 0-100 scale with no comment-weight term.
For the verifysoft variant (with comment weight), use `cm miv` instead.

Formula:
  MI = MAX(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)) * 100 / 171)

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines.

Thresholds:
  20-100  green   -- good maintainability
  10-19   yellow  -- moderate maintainability
  0-9     red     -- low maintainability")]
    Mi {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Generate a comprehensive report combining all code metrics
    Report {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files per section (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,

        /// Show all files instead of truncating to top N
        #[arg(long)]
        full: bool,
    },

    /// Compute Maintainability Index per file (verifysoft variant, with comment weight)
    #[command(long_about = "\
Compute Maintainability Index (MI) per file using the verifysoft.com variant.

This variant includes a comment-weight term (MIcw) that rewards well-commented
code. For the simpler Visual Studio variant (0-100 scale, no comment weight),
use `cm mi` instead.

Formula:
  MIwoc = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
  MIcw  = 50 * sin(sqrt(2.46 * radians(PerCM)))
  MI    = MIwoc + MIcw

Where V = Halstead Volume, G = cyclomatic complexity,
LOC = code lines, PerCM = comment percentage (converted to radians).

Thresholds:
  85+     good         -- easy to maintain
  65-84   moderate     -- reasonable maintainability
  <65     difficult    -- hard to maintain")]
    Miv {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: mi, volume, complexity, or loc (default: mi)
        #[arg(long, default_value = "mi", value_parser = ["mi", "volume", "complexity", "loc"])]
        sort_by: String,
    },

    /// Find hotspots: files that change frequently and have high complexity
    #[command(long_about = "\
Find hotspots: files that change frequently AND have high complexity.

Based on Adam Thornhill's method (\"Your Code as a Crime Scene\"):
  Score = commits × complexity

By default, complexity is measured by total indentation (Thornhill's original
method). Use --complexity cycom for cyclomatic complexity instead.

Files with high scores are both change-prone and complex — they concentrate
risk and are the highest-value refactoring targets.

Requires a git repository. Use --since to limit the analysis window
(approximations: 1 month = 30 days, 1 year = 365 days).

Examples:
  cm hotspots                    # indentation complexity (default)
  cm hotspots --complexity cycom # cyclomatic complexity
  cm hotspots --since 6m         # last 6 months
  cm hotspots --since 1y --sort-by commits
  cm hotspots --json             # machine-readable output")]
    Hotspots {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by metric: score, commits, or complexity (default: score)
        #[arg(long, default_value = "score", value_parser = ["score", "commits", "complexity"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Complexity metric: indent (default, Thornhill) or cycom (cyclomatic)
        #[arg(long, default_value = "indent", value_parser = ["indent", "cycom"])]
        complexity: String,
    },

    /// Analyze code ownership patterns via git blame (knowledge maps)
    #[command(long_about = "\
Analyze code ownership patterns via git blame (knowledge maps).

Based on Adam Thornhill's method (\"Your Code as a Crime Scene\" caps 8-9):
identifies bus factor risk and knowledge concentration per file.

Risk levels:
  CRITICAL  -- one person owns >80% of the code
  HIGH      -- one person owns 60-80%
  MEDIUM    -- 2-3 people own >80% combined
  LOW       -- well-distributed ownership

Use --since to detect knowledge loss: files where the primary owner
has not committed recently. Use --risk-only to show only those files.

Requires a git repository. Generated files (lock files, minified JS, etc.)
are automatically excluded.

Examples:
  cm knowledge                          # ownership by concentration
  cm knowledge --sort-by risk           # highest risk first
  cm knowledge --since 6m --risk-only   # knowledge loss detection
  cm knowledge --json                   # machine-readable output")]
    Knowledge {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N files (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by: concentration, diffusion, or risk (default: concentration)
        #[arg(long, default_value = "concentration", value_parser = ["concentration", "diffusion", "risk"])]
        sort_by: String,

        /// Only consider recent activity since this time for knowledge loss detection (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Show only files with knowledge loss risk (primary owner inactive)
        #[arg(long)]
        risk_only: bool,
    },

    /// Analyze temporal coupling: files that change together in commits
    #[command(long_about = "\
Analyze temporal coupling between files via git history.

Based on Adam Thornhill's method (\"Your Code as a Crime Scene\" ch. 7):
files that frequently change together in the same commits have implicit
coupling, even without direct imports.

Coupling strength = shared_commits / min(commits_a, commits_b)

Levels:
  STRONG    -- strength >= 0.5 (files change together most of the time)
  MODERATE  -- strength 0.3-0.5
  WEAK      -- strength < 0.3

High coupling between unrelated modules suggests hidden dependencies
or architectural issues — consider extracting shared abstractions.

Requires a git repository. File renames are not tracked across history.

Examples:
  cm tc                          # default: min 3 shared commits
  cm tc --min-degree 5           # stricter filter
  cm tc --since 6m               # last 6 months only
  cm tc --min-strength 0.5       # only strong coupling
  cm tc --json                   # machine-readable output")]
    Tc {
        /// Directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Show only the top N file pairs (default: 20)
        #[arg(long, default_value = "20")]
        top: usize,

        /// Sort by: strength or shared (default: strength)
        #[arg(long, default_value = "strength", value_parser = ["strength", "shared"])]
        sort_by: String,

        /// Only consider commits since this time (e.g. 6m, 1y, 30d)
        #[arg(long)]
        since: Option<String>,

        /// Minimum commits per file to be included (default: 3)
        #[arg(long, default_value = "3")]
        min_degree: usize,

        /// Filter results: show only pairs with strength >= threshold (e.g. 0.5 for strong coupling only)
        #[arg(long)]
        min_strength: Option<f64>,
    },

    /// Compute an overall code health score for the project (A++ to F--)
    #[command(long_about = "\
Compute an overall code health score for the project.

Analyzes 6 dimensions of code quality and produces a letter grade
from A++ (exceptional) to F-- (severe issues). Each dimension is
scored 0-100 and weighted to produce a final project score.

Dimensions and weights:
  Maintainability Index   30%  (verifysoft MI, normalized to 0-100)
  Cyclomatic Complexity   20%  (max complexity per file)
  Duplication             15%  (project-wide duplicate code %)
  Indentation Complexity  15%  (stddev of indentation depth)
  Halstead Effort         15%  (mental effort per LOC)
  File Size                5%  (optimal 50-300 LOC)

Non-code files (Markdown, TOML, JSON, etc.) are automatically excluded.
Inline test blocks (#[cfg(test)]) are excluded from duplication analysis.

Grade scale:
  A++ (97-100)  A+ (93-96)  A (90-92)  A- (87-89)
  B+  (83-86)   B  (80-82)  B- (77-79)
  C+  (73-76)   C  (70-72)  C- (67-69)
  D+  (63-66)   D  (60-62)  D- (57-59)
  F   (40-56)   F-- (0-39)

The report includes a breakdown by dimension and a list of files
that need the most attention (lowest per-file scores).

Uses only static code metrics (no git history required).

Examples:
  cm score                       # score current directory
  cm score src/                  # score a subdirectory
  cm score --json                # machine-readable output
  cm score --bottom 20           # show 20 worst files
  cm score --include-tests       # include test files")]
    Score {
        /// File or directory to analyze (default: current directory)
        path: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include test files and directories in analysis (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Number of worst files to show in "needs attention" (default: 10)
        #[arg(long, default_value = "10")]
        bottom: usize,

        /// Minimum lines for a duplicate block (default: 6)
        #[arg(long, default_value = "6")]
        min_lines: usize,
    },

    /// AI-powered code analysis and tooling
    Ai {
        #[command(subcommand)]
        command: AiCommands,
    },
}

#[derive(Subcommand)]
enum AiCommands {
    /// Analyze repository using an AI provider
    #[command(long_about = "\
Analyze a repository using an AI provider.

Invokes an AI model that uses cm tools to analyze the repository and produce
a comprehensive report including code health, complexity hotspots,
maintainability issues, and actionable recommendations.

Supported providers:
  claude  — Anthropic Claude (requires ANTHROPIC_API_KEY env var)

Examples:
  cm ai analyze claude                           # analyze current directory
  cm ai analyze claude src/                      # analyze a subdirectory
  cm ai analyze claude --model claude-sonnet-4-5-20250929  # use specific model
  cm ai analyze claude --output report.md       # save report to file")]
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

    /// Install a Claude Code skill for cm
    #[command(long_about = "\
Install a Claude Code skill that enables Claude Code to use cm for code analysis.

The skill teaches Claude Code how to run cm subcommands and interpret
their JSON output to produce comprehensive code analysis reports.

No API key is needed — Claude Code itself acts as the LLM.

Supported providers:
  claude  — installs a Claude Code skill

Examples:
  cm ai skill claude                    # install the skill")]
    Skill {
        /// Provider for the skill (e.g. claude)
        provider: String,
    },
}

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Loc {
            path,
            verbose,
            json,
        } => run_command(path, |t| loc::run(t, verbose, json)),
        Commands::Dups {
            path,
            report,
            show_all,
            min_lines,
            json,
            include_tests,
        } => run_command(path, |t| {
            dups::run(t, min_lines, report, show_all, json, !include_tests)
        }),
        Commands::Indent {
            path,
            json,
            include_tests,
        } => run_command(path, |t| indent::run(t, json, include_tests)),
        Commands::Hal {
            path,
            json,
            include_tests,
            top,
            sort_by,
        } => run_command(path, |t| hal::run(t, json, include_tests, top, &sort_by)),
        Commands::Cycom {
            path,
            json,
            include_tests,
            min_complexity,
            top,
            per_function,
            sort_by,
        } => run_command(path, |t| {
            cycom::run(
                t,
                json,
                include_tests,
                min_complexity,
                top,
                per_function,
                &sort_by,
            )
        }),
        Commands::Mi {
            path,
            json,
            include_tests,
            top,
            sort_by,
        } => run_command(path, |t| mi::run(t, json, include_tests, top, &sort_by)),
        Commands::Report {
            path,
            json,
            include_tests,
            top,
            min_lines,
            full,
        } => {
            let effective_top = if full { usize::MAX } else { top };
            run_command(path, |t| {
                report::run(t, json, include_tests, effective_top, min_lines)
            });
        }
        Commands::Miv {
            path,
            json,
            include_tests,
            top,
            sort_by,
        } => run_command(path, |t| miv::run(t, json, include_tests, top, &sort_by)),
        Commands::Hotspots {
            path,
            json,
            include_tests,
            top,
            sort_by,
            since,
            complexity,
        } => run_command(path, |t| {
            hotspots::run(
                t,
                json,
                include_tests,
                top,
                &sort_by,
                since.as_deref(),
                &complexity,
            )
        }),
        Commands::Knowledge {
            path,
            json,
            include_tests,
            top,
            sort_by,
            since,
            risk_only,
        } => run_command(path, |t| {
            knowledge::run(
                t,
                json,
                include_tests,
                top,
                &sort_by,
                since.as_deref(),
                risk_only,
            )
        }),
        Commands::Tc {
            path,
            json,
            include_tests,
            top,
            sort_by,
            since,
            min_degree,
            min_strength,
        } => run_command(path, |t| {
            tc::run(
                t,
                json,
                include_tests,
                top,
                &sort_by,
                since.as_deref(),
                min_degree,
                min_strength,
            )
        }),
        Commands::Score {
            path,
            json,
            include_tests,
            bottom,
            min_lines,
        } => run_command(path, |t| {
            score::run(t, json, include_tests, bottom, min_lines)
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
