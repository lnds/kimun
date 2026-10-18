//! `km score --experimental`: the score with the Maintainability Index
//! (Visual Studio variant) in place of Halstead Effort, next to the score of
//! the default model, so the effect of the change can be seen on a project.

use std::error::Error;

use crate::cli::OutputMode;
use crate::walk::WalkConfig;

use super::analyzer::ProjectScore;
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
    let measured = measure(cfg, min_lines, &ScoringModel::ExperimentalMi);
    let baseline = snapshot_of(&measured, bottom, &ScoringModel::Cognitive).score;
    let experimental = snapshot_of(&measured, bottom, &ScoringModel::ExperimentalMi).score;
    match output {
        OutputMode::Json => return print_json_against(&experimental, target, Some(&baseline)),
        OutputMode::Short => {
            print_short(&experimental);
            println!("baseline s:{:.1} g:{}", baseline.score, baseline.grade);
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

/// What the experimental model changes, and how the score moves.
pub fn comparison(baseline: &ProjectScore, experimental: &ProjectScore) -> String {
    format!(
        "\n Experimental: Maintainability Index (Visual Studio) in place of Halstead Effort.\n \
         Default model: {} ({:.1})  \u{2192}  experimental: {} ({:.1}), {:+.1}\n",
        baseline.grade,
        baseline.score,
        experimental.grade,
        experimental.score,
        experimental.score - baseline.score,
    )
}

#[cfg(test)]
#[path = "experimental_test.rs"]
mod tests;
