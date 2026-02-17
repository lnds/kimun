/// AI provider skill installer.
///
/// Installs a Claude Code skill file (`SKILL.md`) that teaches the AI
/// provider how to invoke the `cm` CLI for code analysis. The skill
/// can be installed at project level (`.claude/skills/`) or user level
/// (`~/.claude/skills/`).
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Skill markdown content embedded at compile time.
/// Describes all available `cm` subcommands and their JSON schemas.
const SKILL_CONTENT: &str = r#"---
name: cm-analyze
description: Analyze a code repository using cm (code metrics) tool
---

# cm — Code Metrics Analysis Skill

You have access to the `cm` CLI tool for comprehensive code analysis. Use it to analyze repositories across multiple dimensions.

## Available Commands

Run these via the Bash tool. Always use `--json` for machine-readable output.

### Lines of Code
```bash
cm loc [PATH] --json
```
Language breakdown: files, blank lines, comment lines, code lines.

### Code Health Score
```bash
cm score [PATH] --json
```
Overall grade (A++ to F--) across 6 dimensions: maintainability, complexity, duplication, indentation, Halstead effort, file size.

### Cyclomatic Complexity
```bash
cm cycom [PATH] --json --top 20
```
Per-file and per-function complexity. High values indicate hard-to-test code.

### Maintainability Index
```bash
cm miv [PATH] --json --top 20
```
Verifysoft variant (with comment weight). Values below 65 are hard to maintain.

### Halstead Complexity
```bash
cm hal [PATH] --json --top 20 --sort-by effort
```
Effort, volume, and estimated bugs per file.

### Indentation Complexity
```bash
cm indent [PATH] --json
```
Indentation depth stddev — high values suggest deeply nested code.

### Duplicate Code
```bash
cm dups [PATH] --json --report
```
Duplicate blocks across the project.

### Hotspots (requires git)
```bash
cm hotspots [PATH] --json --top 20
```
Files that change frequently AND have high complexity — top refactoring targets.

### Code Ownership (requires git)
```bash
cm knowledge [PATH] --json --top 20
```
Bus factor risk per file via git blame analysis.

### Temporal Coupling (requires git)
```bash
cm tc [PATH] --json --top 20
```
Files that change together in commits — hidden dependencies.

## Analysis Workflow

1. Start with `cm score` for the overall health grade
2. Run `cm loc` for project size and language breakdown
3. Use `cm cycom` and `cm miv` to find the most complex/unmaintainable files
4. Run `cm hotspots` to find high-risk change-prone files
5. Check `cm dups` for code duplication opportunities
6. Optionally run `cm knowledge` and `cm tc` for team/architecture insights

## Output Format

Produce a structured report with:
- **Overview**: Project size, languages, overall grade
- **Code Health**: Score breakdown by dimension
- **Complexity Hotspots**: Worst files by complexity
- **Maintainability**: Files hardest to maintain
- **Key Findings**: Notable patterns and risks
- **Recommendations**: Prioritized, actionable suggestions

Reference specific file names and metric values. Be concise but thorough.
"#;

/// Install the `cm-analyze` skill for the given AI provider.
///
/// Prompts the user to choose between project-level and user-level
/// installation, creates the directory structure, and writes the skill file.
/// Currently only the `claude` provider is supported.
pub fn install(provider: &str) -> Result<(), Box<dyn std::error::Error>> {
    if provider != "claude" {
        return Err(format!("Unsupported provider: {provider}. Supported: claude").into());
    }

    println!("Where do you want to install the cm skill?");
    println!("  1) Project-level (.claude/skills/cm-analyze/)");
    println!("  2) User-level (~/.claude/skills/cm-analyze/)");
    print!("Choose [1/2]: ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let choice = choice.trim();

    let skill_dir = match choice {
        "1" => PathBuf::from(".claude/skills/cm-analyze"),
        "2" => {
            let home = std::env::var("HOME").map_err(|_| "Could not determine home directory")?;
            PathBuf::from(home).join(".claude/skills/cm-analyze")
        }
        _ => return Err("Invalid choice. Please enter 1 or 2.".into()),
    };

    fs::create_dir_all(&skill_dir)?;
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, SKILL_CONTENT)?;

    println!("Skill installed at: {}", skill_path.display());
    println!("Claude Code will now be able to use cm for code analysis.");

    Ok(())
}
