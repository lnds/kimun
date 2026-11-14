//! `km score --experimental`: the score with the Maintainability Index
//! (Visual Studio variant) in place of Halstead Effort, next to the score of
//! the default model, so the effect of the change can be seen on a project.

use std::error::Error;

use crate::cli::OutputMode;
use crate::walk::WalkConfig;

use super::analyzer::{Graded, ProjectScore, grade_text, score_text};
use super::report::{print_json_against, print_report, print_short, print_terse};
use super::{ScoringModel, measure, snapshot_of};

/// Score the project with the experimental model and the default one, from
/// one measurement, and print the first against the second.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    bottom: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let target = cfg.path.to_str().filter(|s| *s != ".");
    let measured = measure(cfg, min_lines, &ScoringModel::ExperimentalMi).readable()?;
    let baseline = snapshot_of(&measured, bottom, &ScoringModel::Cognitive).score;
    let experimental = snapshot_of(&measured, bottom, &ScoringModel::ExperimentalMi).score;
    match output {
        OutputMode::Json => return print_json_against(&experimental, target, Some(&baseline)),
        OutputMode::Short => {
            print_short(&experimental);
            println!(
                "baseline s:{} g:{}",
                score_text(baseline.graded, 1),
                grade_text(baseline.graded)
            );
        }
        OutputMode::Terse => print_terse(&experimental),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => {
            print_report(&experimental, bottom, target);
            print!("{}", comparison(&baseline, &experimental));
        }
    }
    Ok(())
}

/// What the experimental model changes, and how the score moves. `-`
/// stands for nothing to grade, which leaves no move to show.
pub fn comparison(baseline: &ProjectScore, experimental: &ProjectScore) -> String {
    let moved = match (baseline.graded, experimental.graded) {
        (Some(b), Some(e)) => format!(", {:+.1}", e.score - b.score),
        _ => String::new(),
    };
    format!(
        "\n Experimental: Maintainability Index (Visual Studio) in place of Halstead Effort.\n \
         Default model: {}  \u{2192}  experimental: {}{moved}\n",
        shown(baseline.graded),
        shown(experimental.graded),
    )
}

/// The grade and the score, as `B+ (83.2)`, or `-`.
fn shown(graded: Option<Graded>) -> String {
    graded.map_or_else(
        || "-".to_string(),
        |g| format!("{} ({:.1})", g.grade, g.score),
    )
}

#[cfg(test)]
#[path = "experimental_test.rs"]
mod tests;
