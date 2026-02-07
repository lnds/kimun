use super::detector::DuplicateGroup;

pub struct DuplicationMetrics {
    pub total_code_lines: usize,
    pub duplicated_lines: usize,
    pub duplicate_groups: usize,
    pub files_with_duplicates: usize,
    pub largest_block: usize,
}

impl DuplicationMetrics {
    pub fn percentage(&self) -> f64 {
        if self.total_code_lines == 0 {
            0.0
        } else {
            (self.duplicated_lines as f64 / self.total_code_lines as f64) * 100.0
        }
    }
}

fn assessment(percentage: f64) -> &'static str {
    if percentage < 3.0 {
        "Excellent"
    } else if percentage < 5.0 {
        "Good"
    } else if percentage < 10.0 {
        "Moderate"
    } else if percentage < 20.0 {
        "High"
    } else {
        "Very High"
    }
}

pub fn print_summary(metrics: &DuplicationMetrics) {
    let separator = "─".repeat(68);
    let pct = metrics.percentage();

    println!("{separator}");
    println!(" Duplication Analysis");
    println!();
    println!(" Total code lines:     {:>42}", metrics.total_code_lines);
    println!(" Duplicated lines:     {:>42}", metrics.duplicated_lines);
    println!(" Duplication:          {:>41.1}%", pct);
    println!();
    println!(" Duplicate groups:     {:>42}", metrics.duplicate_groups);
    println!(" Files with duplicates:{:>42}", metrics.files_with_duplicates);
    if metrics.largest_block > 0 {
        println!(
            " Largest duplicate:    {:>37} lines",
            metrics.largest_block
        );
    }
    println!();
    println!(" Assessment:           {:>42}", assessment(pct));
    println!("{separator}");
}

pub fn print_detailed(
    metrics: &DuplicationMetrics,
    groups: &[DuplicateGroup],
    show_all: bool,
) {
    print_summary(metrics);

    if groups.is_empty() {
        return;
    }

    let limit = if show_all { groups.len() } else { 20.min(groups.len()) };

    let separator = "─".repeat(68);

    println!();
    println!(" Duplicate Groups (sorted by duplicated lines)");

    for (i, group) in groups.iter().take(limit).enumerate() {
        println!();
        println!("{separator}");
        println!(
            " [{}] {} lines, {} occurrences ({} duplicated lines)",
            i + 1,
            group.line_count,
            group.locations.len(),
            group.duplicated_lines()
        );
        println!();
        for loc in &group.locations {
            println!(
                "   {}:{}-{}",
                loc.file_path.display(),
                loc.start_line,
                loc.end_line
            );
        }
        if !group.sample.is_empty() {
            println!();
            println!(" Sample:");
            for line in &group.sample {
                println!("   {line}");
            }
            if group.line_count > group.sample.len() {
                println!("   ...");
            }
        }
    }

    println!("{separator}");

    if !show_all && limit < groups.len() {
        println!();
        println!(
            " Showing top {} of {} duplicate groups.",
            limit,
            groups.len()
        );
        println!(" Use --show-all to see all groups.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dups::detector::DuplicateLocation;
    use std::path::PathBuf;

    fn sample_metrics() -> DuplicationMetrics {
        DuplicationMetrics {
            total_code_lines: 1000,
            duplicated_lines: 48,
            duplicate_groups: 2,
            files_with_duplicates: 3,
            largest_block: 12,
        }
    }

    fn sample_groups() -> Vec<DuplicateGroup> {
        vec![
            DuplicateGroup {
                locations: vec![
                    DuplicateLocation {
                        file_path: PathBuf::from("src/foo.rs"),
                        start_line: 10,
                        end_line: 21,
                    },
                    DuplicateLocation {
                        file_path: PathBuf::from("src/bar.rs"),
                        start_line: 30,
                        end_line: 41,
                    },
                ],
                line_count: 12,
                sample: vec![
                    "fn process() {".to_string(),
                    "let x = read();".to_string(),
                    "transform(x);".to_string(),
                ],
            },
            DuplicateGroup {
                locations: vec![
                    DuplicateLocation {
                        file_path: PathBuf::from("src/a.rs"),
                        start_line: 1,
                        end_line: 6,
                    },
                    DuplicateLocation {
                        file_path: PathBuf::from("src/b.rs"),
                        start_line: 5,
                        end_line: 10,
                    },
                    DuplicateLocation {
                        file_path: PathBuf::from("src/c.rs"),
                        start_line: 20,
                        end_line: 25,
                    },
                ],
                line_count: 6,
                sample: vec!["use std::io;".to_string(), "use std::fs;".to_string()],
            },
        ]
    }

    #[test]
    fn percentage_zero_lines() {
        let m = DuplicationMetrics {
            total_code_lines: 0,
            duplicated_lines: 0,
            duplicate_groups: 0,
            files_with_duplicates: 0,
            largest_block: 0,
        };
        assert_eq!(m.percentage(), 0.0);
    }

    #[test]
    fn percentage_calculation() {
        let m = sample_metrics();
        assert!((m.percentage() - 4.8).abs() < 0.01);
    }

    #[test]
    fn assessment_labels() {
        assert_eq!(assessment(0.0), "Excellent");
        assert_eq!(assessment(2.9), "Excellent");
        assert_eq!(assessment(3.0), "Good");
        assert_eq!(assessment(4.9), "Good");
        assert_eq!(assessment(5.0), "Moderate");
        assert_eq!(assessment(9.9), "Moderate");
        assert_eq!(assessment(10.0), "High");
        assert_eq!(assessment(19.9), "High");
        assert_eq!(assessment(20.0), "Very High");
        assert_eq!(assessment(50.0), "Very High");
    }

    #[test]
    fn print_summary_does_not_panic() {
        print_summary(&sample_metrics());
    }

    #[test]
    fn print_summary_zero_metrics() {
        let m = DuplicationMetrics {
            total_code_lines: 0,
            duplicated_lines: 0,
            duplicate_groups: 0,
            files_with_duplicates: 0,
            largest_block: 0,
        };
        print_summary(&m);
    }

    #[test]
    fn print_detailed_does_not_panic() {
        print_detailed(&sample_metrics(), &sample_groups(), false);
    }

    #[test]
    fn print_detailed_show_all() {
        print_detailed(&sample_metrics(), &sample_groups(), true);
    }

    #[test]
    fn print_detailed_empty_groups() {
        let m = DuplicationMetrics {
            total_code_lines: 100,
            duplicated_lines: 0,
            duplicate_groups: 0,
            files_with_duplicates: 0,
            largest_block: 0,
        };
        print_detailed(&m, &[], false);
    }
}
