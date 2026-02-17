use serde::Serialize;

use super::analyzer::{Grade, ProjectScore};
use crate::report_helpers;

/// Print the project health score as a formatted table, showing per-dimension
/// breakdown and the worst-scoring files that need attention.
pub fn print_report(score: &ProjectScore, bottom: usize, target: Option<&str>) {
    let separator = report_helpers::separator(66);

    let header = match target {
        Some(t) if score.files_analyzed == 1 => format!("File Score: {t}"),
        Some(t) => format!("Code Health Score: {t}"),
        None => "Code Health Score".to_string(),
    };
    println!("{header}");
    println!("{separator}");
    println!(" Project Score:  {} ({:.1})", score.grade, score.score);
    println!(" Files Analyzed: {}", score.files_analyzed);
    println!(" Total LOC:      {}", format_thousands(score.total_loc));
    println!("{separator}");
    println!(
        " {:<25} {:>6}   {:>5}   {:<5}",
        "Dimension", "Weight", "Score", "Grade"
    );
    println!("{separator}");

    for d in &score.dimensions {
        println!(
            " {:<25} {:>5.0}%   {:>5.1}   {:<5}",
            d.name,
            d.weight * 100.0,
            d.score,
            d.grade.as_str(),
        );
    }

    println!("{separator}");

    if score.needs_attention.is_empty() {
        return;
    }

    let show = bottom.min(score.needs_attention.len());
    println!();
    println!(" Files Needing Attention (worst per-file scores, excl. duplication)");
    println!("{separator}");

    let max_path = score.needs_attention[..show]
        .iter()
        .map(|f| f.path.display().to_string().len())
        .max()
        .unwrap() // safe: show > 0 because needs_attention is non-empty
        .clamp(4, 40);

    println!(
        " {:>5}  {:<5}  {:<width$}   Issues",
        "Score",
        "Grade",
        "File",
        width = max_path
    );
    println!("{separator}");

    for f in &score.needs_attention[..show] {
        let path_str = f.path.display().to_string();
        let truncated = if path_str.len() > max_path {
            format!("...{}", &path_str[path_str.len() - max_path + 3..])
        } else {
            path_str
        };
        println!(
            " {:>5.1}  {:<5}  {:<width$}   {}",
            f.score,
            f.grade.as_str(),
            truncated,
            f.issues.join(", "),
            width = max_path
        );
    }

    println!("{separator}");
}

/// Format an integer with thousand separators (e.g. 1234567 → "1,234,567").
fn format_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// JSON-serializable representation of a single score dimension.
#[derive(Serialize)]
struct JsonDimension {
    name: String,
    weight: f64,
    score: f64,
    grade: Grade,
}

/// JSON-serializable representation of a per-file score with issues.
#[derive(Serialize)]
struct JsonFileScore {
    path: String,
    score: f64,
    grade: Grade,
    issues: Vec<String>,
}

/// JSON-serializable representation of the full project score output.
#[derive(Serialize)]
struct JsonProjectScore {
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    score: f64,
    grade: Grade,
    files_analyzed: usize,
    total_loc: usize,
    dimensions: Vec<JsonDimension>,
    needs_attention: Vec<JsonFileScore>,
}

/// Serialize the project score to pretty-printed JSON and print to stdout.
pub fn print_json(
    score: &ProjectScore,
    target: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = JsonProjectScore {
        target: target.map(String::from),
        score: score.score,
        grade: score.grade,
        files_analyzed: score.files_analyzed,
        total_loc: score.total_loc,
        dimensions: score
            .dimensions
            .iter()
            .map(|d| JsonDimension {
                name: d.name.to_string(),
                weight: d.weight,
                score: d.score,
                grade: d.grade,
            })
            .collect(),
        needs_attention: score
            .needs_attention
            .iter()
            .map(|f| JsonFileScore {
                path: f.path.display().to_string(),
                score: f.score,
                grade: f.grade,
                issues: f.issues.clone(),
            })
            .collect(),
    };
    report_helpers::print_json_stdout(&json)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
