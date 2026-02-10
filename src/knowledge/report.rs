use serde::Serialize;

use super::analyzer::FileOwnership;

pub fn print_report(files: &[FileOwnership]) {
    if files.is_empty() {
        println!("No files found for knowledge map analysis.");
        return;
    }

    let max_path_len = files
        .iter()
        .map(|f| f.path.display().to_string().len())
        .max()
        .unwrap_or(4)
        .max(4);

    let max_owner_len = files
        .iter()
        .map(|f| f.primary_owner.len())
        .max()
        .unwrap_or(5)
        .max(5);

    // path + 2 + lang(10) + 1 + lines(7) + 1 + owner + 1 + own%(5) + 1 + contrib(7) + 1 + risk(8) + 1
    let header_width = max_path_len + max_owner_len + 45;
    let separator = "─".repeat(header_width.max(78));

    println!("Knowledge Map — Code Ownership");
    println!("{separator}");
    println!(
        " {:<pw$}  {:>10} {:>7}  {:<ow$} {:>5} {:>7} {:>8}",
        "File",
        "Language",
        "Lines",
        "Owner",
        "Own%",
        "Contrib",
        "Risk",
        pw = max_path_len,
        ow = max_owner_len
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<pw$}  {:>10} {:>7}  {:<ow$} {:>4.0}% {:>7} {:>8}",
            f.path.display(),
            f.language,
            f.total_lines,
            f.primary_owner,
            f.ownership_pct,
            f.contributors,
            f.risk.label(),
            pw = max_path_len,
            ow = max_owner_len
        );
    }

    println!("{separator}");

    let loss_count = files.iter().filter(|f| f.knowledge_loss).count();
    if loss_count > 0 {
        println!();
        println!("Files with knowledge loss risk (primary owner inactive): {loss_count}");
        for f in files.iter().filter(|f| f.knowledge_loss) {
            println!("  {} ({})", f.path.display(), f.primary_owner);
        }
    }
}

#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    total_lines: usize,
    primary_owner: String,
    ownership_pct: f64,
    contributors: usize,
    risk: String,
    knowledge_loss: bool,
}

pub fn print_json(files: &[FileOwnership]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| JsonEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            total_lines: f.total_lines,
            primary_owner: f.primary_owner.clone(),
            ownership_pct: (f.ownership_pct * 10.0).round() / 10.0,
            contributors: f.contributors,
            risk: f.risk.label().to_string(),
            knowledge_loss: f.knowledge_loss,
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::analyzer::RiskLevel;
    use std::path::PathBuf;

    fn sample_files() -> Vec<FileOwnership> {
        vec![
            FileOwnership {
                path: PathBuf::from("src/foo.rs"),
                language: "Rust".to_string(),
                total_lines: 731,
                primary_owner: "Alice".to_string(),
                ownership_pct: 94.0,
                contributors: 2,
                risk: RiskLevel::Critical,
                knowledge_loss: true,
            },
            FileOwnership {
                path: PathBuf::from("src/bar.rs"),
                language: "Rust".to_string(),
                total_lines: 241,
                primary_owner: "Bob".to_string(),
                ownership_pct: 78.0,
                contributors: 3,
                risk: RiskLevel::High,
                knowledge_loss: false,
            },
        ]
    }

    #[test]
    fn print_report_does_not_panic() {
        print_report(&sample_files());
    }

    #[test]
    fn print_report_empty() {
        print_report(&[]);
    }

    #[test]
    fn print_json_does_not_panic() {
        print_json(&sample_files()).unwrap();
    }

    #[test]
    fn print_json_empty() {
        print_json(&[]).unwrap();
    }
}
