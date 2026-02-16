use serde::Serialize;

use super::analyzer::{Grade, ProjectScore};

pub fn print_report(score: &ProjectScore, bottom: usize, target: Option<&str>) {
    let separator = "\u{2500}".repeat(66);

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

#[derive(Serialize)]
struct JsonDimension {
    name: String,
    weight: f64,
    score: f64,
    grade: Grade,
}

#[derive(Serialize)]
struct JsonFileScore {
    path: String,
    score: f64,
    grade: Grade,
    issues: Vec<String>,
}

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
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::score::analyzer::{DimensionScore, FileScore};

    fn sample_score() -> ProjectScore {
        ProjectScore {
            score: 84.3,
            grade: Grade::BPlus,
            files_analyzed: 42,
            total_loc: 8432,
            dimensions: vec![
                DimensionScore {
                    name: "Maintainability Index",
                    weight: 0.25,
                    score: 88.2,
                    grade: Grade::AMinus,
                },
                DimensionScore {
                    name: "Cyclomatic Complexity",
                    weight: 0.20,
                    score: 82.4,
                    grade: Grade::BPlus,
                },
            ],
            needs_attention: vec![FileScore {
                path: PathBuf::from("src/legacy/parser.rs"),
                score: 54.2,
                grade: Grade::F,
                loc: 500,
                issues: vec!["Complexity: 87".to_string(), "MI: 12".to_string()],
            }],
        }
    }

    fn empty_score() -> ProjectScore {
        ProjectScore {
            score: 0.0,
            grade: Grade::FMinusMinus,
            files_analyzed: 0,
            total_loc: 0,
            dimensions: vec![],
            needs_attention: vec![],
        }
    }

    #[test]
    fn print_report_does_not_panic() {
        print_report(&sample_score(), 10, None);
    }

    #[test]
    fn print_report_empty() {
        print_report(&empty_score(), 10, None);
    }

    #[test]
    fn print_report_with_target_dir() {
        print_report(&sample_score(), 10, Some("src/"));
    }

    #[test]
    fn print_report_with_target_file() {
        let mut score = sample_score();
        score.files_analyzed = 1;
        print_report(&score, 10, Some("src/main.rs"));
    }

    #[test]
    fn print_json_does_not_panic() {
        print_json(&sample_score(), None).unwrap();
    }

    #[test]
    fn print_json_empty() {
        print_json(&empty_score(), None).unwrap();
    }

    #[test]
    fn print_json_with_target() {
        print_json(&sample_score(), Some("src/")).unwrap();
    }

    #[test]
    fn format_thousands_works() {
        assert_eq!(format_thousands(0), "0");
        assert_eq!(format_thousands(999), "999");
        assert_eq!(format_thousands(1000), "1,000");
        assert_eq!(format_thousands(1234567), "1,234,567");
    }
}
