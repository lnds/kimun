//! `km init` — calibrate and write a `.kimun.toml` from current project state.
//!
//! Walks source files to measure average function length and parameter count,
//! runs duplication detection to get the current dup ratio, and computes the
//! health score grade. Proposes thresholds slightly above current state so the
//! project passes immediately but has a concrete improvement target.

use std::error::Error;
use std::io::{self, Write as _};
use std::path::Path;

use crate::config::{DupsConfig, SmellsConfig};
use crate::cycom::markers::markers_for;
use crate::detection::{FunctionDetectionMarkers as _, detect_function_bodies};
use crate::dups;
use crate::loc::counter::LineKind;
use crate::score::{self, ScoringModel, analyzer::Grade};
use crate::util::read_and_classify;
use crate::walk::{ExcludeFilter, WalkConfig};

/// Raw stats collected from the project.
struct ProjectStats {
    avg_function_len: f64,
    avg_param_count: f64,
    function_count: usize,
    dup_ratio: f64,
    health_grade: Grade,
}

pub fn run(path: &Path, yes: bool) -> Result<(), Box<dyn Error>> {
    let output_path = path.join(".kimun.toml");
    if output_path.exists() && !yes {
        eprintln!(
            "warning: .kimun.toml already exists at {}",
            output_path.display()
        );
        eprint!("Overwrite? [y/N] ");
        io::stderr().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(path, false, &filter);

    print!("Analyzing project...");
    io::stdout().flush()?;

    let stats = collect_stats(&cfg)?;

    println!(" done.\n");

    let suggested_max_lines = suggest_max_lines(stats.avg_function_len);
    let suggested_max_params = suggest_max_params(stats.avg_param_count);
    let suggested_max_dup_ratio = suggest_max_dup_ratio(stats.dup_ratio);
    let suggested_fail_below = grade_one_lower(stats.health_grade);

    let show_params = stats.function_count > 0 && stats.avg_param_count >= 1.0;

    let fn_val = format!("{:.0} lines", stats.avg_function_len);
    let param_val = format!("{:.1}", stats.avg_param_count);
    let dup_val = format!("{:.1}%", stats.dup_ratio);
    let grade_val = stats.health_grade.as_str();

    let mut widths = vec![dup_val.len(), grade_val.len()];
    if stats.function_count > 0 {
        widths.push(fn_val.len());
    }
    if show_params {
        widths.push(param_val.len());
    }
    let val_width = widths.into_iter().max().unwrap_or(0);

    println!("Current state:");
    if stats.function_count > 0 {
        println!(
            "  avg function length: {fn_val:<val_width$}  →  suggested max_lines = {suggested_max_lines}"
        );
    }
    if show_params {
        println!(
            "  avg param count:     {param_val:<val_width$}  →  suggested max_params = {suggested_max_params}"
        );
    }
    println!(
        "  dup ratio:           {dup_val:<val_width$}  →  suggested max_dup_ratio = {suggested_max_dup_ratio:.1}"
    );
    println!(
        "  health score:        {grade_val:<val_width$}  →  suggested fail_below = {}",
        suggested_fail_below.as_str()
    );
    println!();

    if !yes {
        eprint!("Write .kimun.toml with these values? [Y/n] ");
        io::stderr().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        if line.trim().eq_ignore_ascii_case("n") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let toml = build_toml(
        suggested_max_lines,
        suggested_max_params,
        suggested_max_dup_ratio,
        suggested_fail_below,
    );

    std::fs::write(&output_path, toml)?;
    println!("Wrote {}", output_path.display());
    Ok(())
}

fn collect_stats(cfg: &WalkConfig<'_>) -> Result<ProjectStats, Box<dyn Error>> {
    let mut all_lengths: Vec<usize> = Vec::new();
    let mut all_params: Vec<usize> = Vec::new();

    for (file_path, spec) in cfg.source_files() {
        let Some(markers) = markers_for(spec.name) else {
            continue;
        };
        let Ok(Some((lines, kinds))) = read_and_classify(&file_path, spec) else {
            continue;
        };

        let code_lines: Vec<(usize, &str)> = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| kinds.get(*i) == Some(&LineKind::Code))
            .map(|(i, l)| (i, l.as_str()))
            .collect();

        let functions = detect_function_bodies(&lines, &code_lines, markers);
        let overhead = if markers.brace_scoped() { 2 } else { 1 };

        for func in &functions {
            let total = func.code_lines.len();
            let body_len = total.saturating_sub(overhead.min(total));
            all_lengths.push(body_len);

            if let Some(sig_line) = lines.get(func.start_line)
                && let Some(n) = count_params(sig_line)
            {
                all_params.push(n);
            }
        }
    }

    let function_count = all_lengths.len();
    let avg_function_len = if function_count == 0 {
        0.0
    } else {
        all_lengths.iter().sum::<usize>() as f64 / function_count as f64
    };
    let avg_param_count = if all_params.is_empty() {
        0.0
    } else {
        all_params.iter().sum::<usize>() as f64 / all_params.len() as f64
    };

    let dup_metrics = dups::compute_metrics(cfg, DupsConfig::DEFAULT_MIN_LINES);
    let dup_ratio = dup_metrics.percentage();

    let score = score::compute_score(cfg, 0, 6, &ScoringModel::Cognitive)?;
    let health_grade = score.grade;

    Ok(ProjectStats {
        avg_function_len,
        avg_param_count,
        function_count,
        dup_ratio,
        health_grade,
    })
}

/// Count parameters in a function signature line.
/// Counts top-level commas only — commas inside nested parens are ignored.
fn count_params(sig: &str) -> Option<usize> {
    let open = sig.find('(')?;
    let after = &sig[open + 1..];
    let mut depth = 1usize;
    let mut top_level_commas = 0usize;
    let mut all_empty = true;
    for ch in after.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ',' if depth == 1 => top_level_commas += 1,
            c if depth == 1 && !c.is_whitespace() => all_empty = false,
            _ => {}
        }
    }
    if all_empty && top_level_commas == 0 {
        Some(0)
    } else {
        Some(top_level_commas + 1)
    }
}

fn suggest_max_lines(avg: f64) -> usize {
    if avg < 1.0 {
        return SmellsConfig::DEFAULT_MAX_LINES;
    }
    // Round avg up to the next multiple of 5, then add one more unit of 5 as headroom.
    let next_multiple = (avg as usize).div_ceil(5) * 5;
    (next_multiple + 5).max(20)
}

fn suggest_max_params(avg: f64) -> usize {
    (avg.ceil() as usize).max(3)
}

fn suggest_max_dup_ratio(ratio: f64) -> f64 {
    if ratio < 0.1 {
        return 2.0;
    }
    // Round up to next 0.5 boundary — stays close to current level to prevent regression.
    ((ratio * 2.0).ceil() / 2.0).max(2.0)
}

fn grade_one_lower(grade: Grade) -> Grade {
    use Grade::*;
    match grade {
        APlusPlus => APlus,
        APlus => A,
        A => AMinus,
        AMinus => BPlus,
        BPlus => B,
        B => BMinus,
        BMinus => CPlus,
        CPlus => C,
        C => CMinus,
        CMinus => DPlus,
        DPlus => D,
        D => DMinus,
        DMinus => F,
        F => FMinus,
        FMinus | FMinusMinus => FMinusMinus,
    }
}

const TEMPLATE: &str = include_str!("template.toml");

fn build_toml(
    max_lines: usize,
    max_params: usize,
    max_dup_ratio: f64,
    fail_below: Grade,
) -> String {
    TEMPLATE
        .replace("{{max_lines}}", &max_lines.to_string())
        .replace("{{max_params}}", &max_params.to_string())
        .replace("{{max_dup_ratio}}", &format!("{max_dup_ratio:.1}"))
        .replace("{{fail_below}}", fail_below.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_max_lines_rounds_up_and_adds_headroom() {
        assert_eq!(suggest_max_lines(38.0), 45); // ceil(38/5)*5 = 40, +5 = 45
        assert_eq!(suggest_max_lines(40.0), 45); // 40 is already multiple, +5 = 45
        assert_eq!(suggest_max_lines(41.0), 50); // ceil(41/5)*5 = 45, +5 = 50
        assert_eq!(suggest_max_lines(0.0), 50); // fallback = DEFAULT_MAX_LINES
    }

    #[test]
    fn suggest_max_params_ceilings_avg() {
        assert_eq!(suggest_max_params(3.2), 4);
        assert_eq!(suggest_max_params(4.0), 4);
        assert_eq!(suggest_max_params(1.5), 3); // minimum 3
    }

    #[test]
    fn suggest_max_dup_ratio_rounds_to_next_half() {
        assert_eq!(suggest_max_dup_ratio(4.1), 4.5); // ceil to next 0.5
        assert_eq!(suggest_max_dup_ratio(8.6), 9.0); // ceil to next 0.5
        assert_eq!(suggest_max_dup_ratio(0.0), 2.0); // minimum
        assert_eq!(suggest_max_dup_ratio(2.0), 2.0); // already on boundary, stays
        assert_eq!(suggest_max_dup_ratio(4.5), 4.5); // already on boundary, stays
    }

    #[test]
    fn grade_one_lower_steps_down() {
        assert_eq!(grade_one_lower(Grade::BPlus), Grade::B);
        assert_eq!(grade_one_lower(Grade::A), Grade::AMinus);
        assert_eq!(grade_one_lower(Grade::FMinusMinus), Grade::FMinusMinus);
    }

    #[test]
    fn count_params_basic() {
        assert_eq!(count_params("fn foo(a: i32, b: i32)"), Some(2));
        assert_eq!(count_params("fn bar()"), Some(0));
        assert_eq!(count_params("fn baz(x: Vec<(i32, i32)>, y: i32)"), Some(2));
    }
}
