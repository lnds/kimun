use serde::Serialize;

use super::analyzer::FileCoupling;

pub fn print_report(pairs: &[FileCoupling], total: usize) {
    if pairs.is_empty() {
        println!("No coupled file pairs found.");
        return;
    }

    let max_a_len = pairs
        .iter()
        .map(|p| p.file_a.display().to_string().len())
        .max()
        .unwrap_or(6)
        .max(6);

    let max_b_len = pairs
        .iter()
        .map(|p| p.file_b.display().to_string().len())
        .max()
        .unwrap_or(6)
        .max(6);

    let header_width = max_a_len + max_b_len + 35;
    let separator = "─".repeat(header_width.max(78));

    println!("Temporal Coupling — Files That Change Together");
    println!("{separator}");
    println!(
        " {:<aw$}  {:<bw$}  {:>6}  {:>8}  {:>8}",
        "File A",
        "File B",
        "Shared",
        "Strength",
        "Level",
        aw = max_a_len,
        bw = max_b_len,
    );
    println!("{separator}");

    for p in pairs {
        println!(
            " {:<aw$}  {:<bw$}  {:>6}  {:>8.2}  {:>8}",
            p.file_a.display(),
            p.file_b.display(),
            p.shared_commits,
            p.strength,
            p.level.label(),
            aw = max_a_len,
            bw = max_b_len,
        );
    }

    println!("{separator}");
    if total > pairs.len() {
        println!();
        println!(
            "{total} coupled pairs found ({shown} shown).",
            shown = pairs.len()
        );
    }
}

#[derive(Serialize)]
struct JsonEntry {
    file_a: String,
    file_b: String,
    shared_commits: usize,
    commits_a: usize,
    commits_b: usize,
    strength: f64,
    level: String,
}

pub fn print_json(pairs: &[FileCoupling]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = pairs
        .iter()
        .map(|p| JsonEntry {
            file_a: p.file_a.display().to_string(),
            file_b: p.file_b.display().to_string(),
            shared_commits: p.shared_commits,
            commits_a: p.commits_a,
            commits_b: p.commits_b,
            strength: (p.strength * 100.0).round() / 100.0,
            level: p.level.label().to_string(),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tc::analyzer::CouplingLevel;
    use std::path::PathBuf;

    fn sample_pairs() -> Vec<FileCoupling> {
        vec![
            FileCoupling {
                file_a: PathBuf::from("src/auth/jwt.rs"),
                file_b: PathBuf::from("src/auth/middleware.rs"),
                shared_commits: 12,
                commits_a: 14,
                commits_b: 14,
                strength: 0.86,
                level: CouplingLevel::Strong,
            },
            FileCoupling {
                file_a: PathBuf::from("lib/parser.rs"),
                file_b: PathBuf::from("lib/validator.rs"),
                shared_commits: 8,
                commits_a: 15,
                commits_b: 10,
                strength: 0.53,
                level: CouplingLevel::Strong,
            },
        ]
    }

    #[test]
    fn print_report_does_not_panic() {
        print_report(&sample_pairs(), 10);
    }

    #[test]
    fn print_report_empty() {
        print_report(&[], 0);
    }

    #[test]
    fn print_json_does_not_panic() {
        print_json(&sample_pairs()).unwrap();
    }

    #[test]
    fn print_json_empty() {
        print_json(&[]).unwrap();
    }
}
