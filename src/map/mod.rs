//! `km map`: an enclosure diagram of the code. Every directory is a circle
//! holding the circles of its subdirectories and files, a file's area
//! grows with its lines of code, and a metric colours the files, so what
//! stands out shows where it sits in the system.

mod callout;
mod enclose;
mod layout;
mod metric;
mod pack;
mod page;

use std::error::Error;
use std::path::{Path, PathBuf};

pub use metric::ColorBy;

use crate::cli::OutputMode;
use crate::score::ScoringModel;
use crate::walk::WalkConfig;

/// What `km map` takes besides the common arguments.
pub struct Options<'a> {
    pub html: Option<&'a Path>,
    pub color_by: ColorBy,
    pub since: Option<&'a str>,
}

/// How the legend names a metric.
fn title(color_by: ColorBy) -> &'static str {
    match color_by {
        ColorBy::Score => "health score",
        ColorBy::Hotspots => "hotspots (commits × complexity)",
        ColorBy::Churn => "churn",
        ColorBy::Age => "age",
        ColorBy::Knowledge => "knowledge loss risk",
        ColorBy::Cycles => "dependency cycles",
        ColorBy::Llm => "readiness for coding agents",
    }
}

pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    opts: &Options<'_>,
) -> Result<(), Box<dyn Error>> {
    match (output, opts.html) {
        (OutputMode::Table, Some(_)) | (OutputMode::Json, None) => {}
        (OutputMode::Table, None) => {
            return Err("km map draws a page: give --html FILE, or --format json".into());
        }
        _ => {
            return Err(
                "km map writes a page with --html, or the laid out tree with --format json".into(),
            );
        }
    }
    let since = opts.since.map(crate::util::parse_since).transpose()?;
    let kcfg = crate::config::KimunConfig::load();
    let model = ScoringModel::from_arg(&kcfg.score.resolve_model(None));
    let measured = crate::score::measure(cfg, kcfg.dups.resolve_min_lines(None), &model);
    let snapshot = crate::score::snapshot_of(&measured, 0, &model);
    let scores = snapshot.files();
    let (mut shades, legend) = metric::shades(cfg, opts.color_by, since, scores)?;
    let files: Vec<(PathBuf, usize, Option<metric::Shade>)> = scores
        .iter()
        .map(|f| {
            let path = f
                .path
                .strip_prefix(cfg.path)
                .unwrap_or(&f.path)
                .to_path_buf();
            let shade = shades.remove(&path);
            (path, f.loc, shade)
        })
        .collect();
    if files.is_empty() {
        return Err("no code files to draw".into());
    }
    let nodes = layout::layout(&files);
    let words = metric::words(opts.color_by);
    let data = page::Data {
        llm: opts.color_by == ColorBy::Llm,
        metric: title(opts.color_by),
        legend: &legend,
        summary: &callout::shares(&nodes, &words),
        callouts: &callout::callouts(&nodes, &words),
        nodes: &nodes,
    };
    let Some(file) = opts.html else {
        return crate::report_helpers::print_json_stdout(&data);
    };
    let root = cfg
        .path
        .canonicalize()
        .unwrap_or_else(|_| cfg.path.to_path_buf());
    let name = root
        .file_name()
        .map_or(String::from("."), |n| n.to_string_lossy().into_owned());
    std::fs::write(file, page::render(&name, &data)?)
        .map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    println!("Wrote {}", file.display());
    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
