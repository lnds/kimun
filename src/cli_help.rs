//! Long help text constants for CLI subcommands.
//!
//! Extracted from `cli.rs` to keep the argument definitions concise
//! and reduce the Halstead token count of the main CLI module.

/// Halstead complexity: operator/operand analysis per file.
/// Shows volume, difficulty, effort, estimated bugs, and development time.
pub const HAL: &str = "\
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

Higher effort/volume/bugs indicate more complex and error-prone code.";

/// Maintainability Index (Visual Studio variant, 0–100 scale).
/// Uses the original formula without comment-weight term.
pub const MI: &str = "\
Compute Maintainability Index (MI) per file using the Visual Studio variant.

This variant normalizes MI to a 0-100 scale with no comment-weight term.
For the verifysoft variant (with comment weight), use `km miv` instead.

Formula:
  MI = MAX(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)) * 100 / 171)

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines.

Thresholds:
  20-100  green   -- good maintainability
  10-19   yellow  -- moderate maintainability
  0-9     red     -- low maintainability";

/// Maintainability Index (verifysoft variant with comment weight).
/// Adds MIcw term that rewards well-commented code.
pub const MIV: &str = "\
Compute Maintainability Index (MI) per file using the verifysoft.com variant.

This variant includes a comment-weight term (MIcw) that rewards well-commented
code. For the simpler Visual Studio variant (0-100 scale, no comment weight),
use `km mi` instead.

Formula:
  MIwoc = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
  MIcw  = 50 * sin(sqrt(2.46 * radians(PerCM)))
  MI    = MIwoc + MIcw

Where V = Halstead Volume, G = cyclomatic complexity,
LOC = code lines, PerCM = comment percentage (converted to radians).

Thresholds:
  85+     good         -- easy to maintain
  65-84   moderate     -- reasonable maintainability
  <65     difficult    -- hard to maintain";

/// Hotspot analysis: files that change frequently AND have high complexity.
/// Based on Adam Thornhill's "Your Code as a Crime Scene" methodology.
pub const HOTSPOTS: &str = "\
Find hotspots: files that change frequently AND have high complexity.

Based on Adam Thornhill's method (\"Your Code as a Crime Scene\"):
  Score = commits \u{00d7} complexity

By default, complexity is measured by total indentation (Thornhill's original
method). Use --complexity cycom for cyclomatic complexity instead.

Files with high scores are both change-prone and complex \u{2014} they concentrate
risk and are the highest-value refactoring targets.

Requires a git repository. Use --since to limit the analysis window
(approximations: 1 month = 30 days, 1 year = 365 days).

Examples:
  km hotspots                    # indentation complexity (default)
  km hotspots --complexity cycom # cyclomatic complexity
  km hotspots --since 6m         # last 6 months
  km hotspots --since 1y --sort-by commits
  km hotspots --json             # machine-readable output";

/// Knowledge maps: code ownership analysis via git blame.
/// Identifies bus factor risk and knowledge concentration per file.
pub const KNOWLEDGE: &str = "\
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
  km knowledge                          # ownership by concentration
  km knowledge --sort-by risk           # highest risk first
  km knowledge --since 6m --risk-only   # knowledge loss detection
  km knowledge --json                   # machine-readable output";

/// Temporal coupling: files that change together in git commits.
/// Reveals hidden dependencies between modules.
pub const TC: &str = "\
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
or architectural issues \u{2014} consider extracting shared abstractions.

Requires a git repository. File renames are not tracked across history.

Examples:
  km tc                          # default: min 3 shared commits
  km tc --min-degree 5           # stricter filter
  km tc --since 6m               # last 6 months only
  km tc --min-strength 0.5       # only strong coupling
  km tc --json                   # machine-readable output";

/// Overall code health score: weighted aggregate of 6 quality dimensions.
/// Produces a letter grade from A++ (exceptional) to F-- (severe issues).
pub const SCORE: &str = "\
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
  km score                       # score current directory
  km score src/                  # score a subdirectory
  km score --json                # machine-readable output
  km score --bottom 20           # show 20 worst files
  km score --include-tests       # include test files";

/// AI-powered repository analysis using an external LLM provider.
/// The model runs km tools and produces a comprehensive report.
pub const AI_ANALYZE: &str = "\
Analyze a repository using an AI provider.

Invokes an AI model that uses km tools to analyze the repository and produce
a comprehensive report including code health, complexity hotspots,
maintainability issues, and actionable recommendations.

Supported providers:
  claude  \u{2014} Anthropic Claude (requires ANTHROPIC_API_KEY env var)

Examples:
  km ai analyze claude                           # analyze current directory
  km ai analyze claude src/                      # analyze a subdirectory
  km ai analyze claude --model claude-sonnet-4-5-20250929  # use specific model
  km ai analyze claude --output report.md       # save report to file";

/// Claude Code skill installer for km integration.
/// Enables Claude Code to use km for code analysis without an API key.
pub const AI_SKILL: &str = "\
Install a Claude Code skill that enables Claude Code to use km for code analysis.

The skill teaches Claude Code how to run km subcommands and interpret
their JSON output to produce comprehensive code analysis reports.

No API key is needed \u{2014} Claude Code itself acts as the LLM.

Supported providers:
  claude  \u{2014} installs a Claude Code skill

Examples:
  km ai skill claude                    # install the skill";
