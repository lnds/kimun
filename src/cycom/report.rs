use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::{CyclomaticLevel, FunctionComplexity};

pub struct FileCycomMetrics {
    pub path: PathBuf,
    pub language: String,
    pub function_count: usize,
    pub avg_complexity: f64,
    pub max_complexity: usize,
    pub total_complexity: usize,
    pub level: CyclomaticLevel,
    pub functions: Vec<FunctionComplexity>,
}

pub fn print_report(files: &[FileCycomMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = files
        .iter()
        .map(|f| f.path.display().to_string().len())
        .max()
        .unwrap_or(4)
        .max(4);

    let header_width = max_path_len + 55;
    let separator = "\u{2500}".repeat(header_width.max(78));

    println!("Cyclomatic Complexity");
    println!("{separator}");
    println!(
        " {:<width$}  {:>9} {:>5} {:>5} {:>7}  Level",
        "File",
        "Functions",
        "Avg",
        "Max",
        "Total",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<width$}  {:>9} {:>5.1} {:>5} {:>7}  {}",
            f.path.display(),
            f.function_count,
            f.avg_complexity,
            f.max_complexity,
            f.total_complexity,
            f.level.as_str(),
            width = max_path_len
        );
    }

    println!("{separator}");

    let total_functions: usize = files.iter().map(|f| f.function_count).sum();
    let total_complexity: usize = files.iter().map(|f| f.total_complexity).sum();
    let max_complexity = files.iter().map(|f| f.max_complexity).max().unwrap_or(0);
    let avg = if total_functions > 0 {
        total_complexity as f64 / total_functions as f64
    } else {
        0.0
    };

    let total_label = format!(" Total ({} files)", files.len());
    println!(
        "{:<width$}  {:>9} {:>5.1} {:>5} {:>7}",
        total_label,
        total_functions,
        avg,
        max_complexity,
        total_complexity,
        width = max_path_len + 1,
    );
}

pub fn print_per_function(files: &[FileCycomMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let separator = "\u{2500}".repeat(78);
    println!("Cyclomatic Complexity (per function)");
    println!("{separator}");

    for f in files {
        println!();
        println!("{}:", f.path.display());

        let max_name_len = f
            .functions
            .iter()
            .map(|func| func.name.len())
            .max()
            .unwrap_or(10)
            .max(10);

        for func in &f.functions {
            println!(
                "  {:<width$}  {:>5}  {}",
                func.name,
                func.complexity,
                func.level.as_str(),
                width = max_name_len
            );
        }
    }

    println!("{separator}");
}

#[derive(Serialize)]
struct JsonFunctionEntry {
    name: String,
    start_line: usize,
    complexity: usize,
    level: CyclomaticLevel,
}

#[derive(Serialize)]
struct JsonFileEntry {
    path: String,
    language: String,
    function_count: usize,
    avg_complexity: f64,
    max_complexity: usize,
    total_complexity: usize,
    level: CyclomaticLevel,
    functions: Vec<JsonFunctionEntry>,
}

pub fn print_json(files: &[FileCycomMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonFileEntry> = files
        .iter()
        .map(|f| JsonFileEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            function_count: f.function_count,
            avg_complexity: f.avg_complexity,
            max_complexity: f.max_complexity,
            total_complexity: f.total_complexity,
            level: f.level,
            functions: f
                .functions
                .iter()
                .map(|func| JsonFunctionEntry {
                    name: func.name.clone(),
                    start_line: func.start_line,
                    complexity: func.complexity,
                    level: func.level,
                })
                .collect(),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_files() -> Vec<FileCycomMetrics> {
        vec![
            FileCycomMetrics {
                path: PathBuf::from("src/foo.rs"),
                language: "Rust".to_string(),
                function_count: 8,
                avg_complexity: 4.2,
                max_complexity: 34,
                total_complexity: 289,
                level: CyclomaticLevel::HighlyComplex,
                functions: vec![
                    FunctionComplexity {
                        name: "classify_reader".to_string(),
                        start_line: 10,
                        complexity: 34,
                        level: CyclomaticLevel::HighlyComplex,
                    },
                    FunctionComplexity {
                        name: "count_lines".to_string(),
                        start_line: 50,
                        complexity: 12,
                        level: CyclomaticLevel::Complex,
                    },
                ],
            },
            FileCycomMetrics {
                path: PathBuf::from("src/bar.rs"),
                language: "Rust".to_string(),
                function_count: 3,
                avg_complexity: 2.0,
                max_complexity: 3,
                total_complexity: 6,
                level: CyclomaticLevel::Simple,
                functions: vec![FunctionComplexity {
                    name: "run".to_string(),
                    start_line: 1,
                    complexity: 3,
                    level: CyclomaticLevel::Simple,
                }],
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
    fn print_per_function_does_not_panic() {
        print_per_function(&sample_files());
    }

    #[test]
    fn print_per_function_empty() {
        print_per_function(&[]);
    }

    #[test]
    fn print_json_does_not_panic() {
        print_json(&sample_files()).unwrap();
    }

    #[test]
    fn print_json_empty() {
        print_json(&[]).unwrap();
    }

    #[test]
    fn json_structure_is_valid() {
        let files = sample_files();
        let entries: Vec<serde_json::Value> = files
            .iter()
            .map(|f| {
                serde_json::json!({
                    "path": f.path.display().to_string(),
                    "language": f.language,
                    "function_count": f.function_count,
                    "avg_complexity": f.avg_complexity,
                    "max_complexity": f.max_complexity,
                    "total_complexity": f.total_complexity,
                    "level": f.level.as_str(),
                    "functions": f.functions.iter().map(|func| {
                        serde_json::json!({
                            "name": func.name,
                            "start_line": func.start_line,
                            "complexity": func.complexity,
                            "level": func.level.as_str(),
                        })
                    }).collect::<Vec<_>>(),
                })
            })
            .collect();

        let json_str = serde_json::to_string_pretty(&entries).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["function_count"], 8);
        assert_eq!(arr[0]["max_complexity"], 34);
        assert!(arr[0]["functions"].is_array());
        assert_eq!(arr[0]["functions"].as_array().unwrap().len(), 2);
    }
}
