//! What colours the files of the map: one metric, read for every file as a
//! heat from -1 (stands out as a problem, red) through 0 (fine, grey) to 1
//! (stands out as healthy or alive, green), with the text a viewer reads.

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::Serialize;

use crate::age::analyzer::{AgeStatus, AgeThresholds};
use crate::churn::analyzer::ChurnLevel;
use crate::git::GitRepo;
use crate::knowledge::analyzer::RiskLevel;
use crate::score::analyzer::FileScore;
use crate::walk::WalkConfig;

/// The metric the map is coloured by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorBy {
    /// Health of each file, as `km score` grades it
    #[default]
    Score,
    /// Change frequency times complexity, as `km hotspots`
    Hotspots,
    /// Change frequency, as `km churn`
    Churn,
    /// Time since the last change, as `km age`
    Age,
    /// Knowledge loss risk, as `km knowledge`
    Knowledge,
    /// Files in a dependency cycle, as `km deps`
    Cycles,
    /// How ready each file is for coding agents, by its `km score`
    Llm,
}

/// How the page speaks of a metric: what a file is on each side of the
/// scale, and the label of the directory where each side gathers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Words {
    pub risk: &'static str,
    pub fine: &'static str,
    pub good: Option<&'static str>,
    pub risk_callout: &'static str,
    pub good_callout: Option<&'static str>,
}

/// The words of `color_by`.
pub fn words(color_by: ColorBy) -> Words {
    let (risk, fine, good, risk_callout, good_callout) = match color_by {
        ColorBy::Score => (
            "unhealthy",
            "fair",
            Some("healthy"),
            "Least healthy code",
            Some("Healthiest code"),
        ),
        ColorBy::Llm => (
            "an LLM risk",
            "needing care",
            Some("LLM-friendly"),
            "Unhealthy code: LLM risk",
            Some("LLM-friendly code"),
        ),
        ColorBy::Hotspots => ("a hotspot", "calm", None, "Hotspots", None),
        ColorBy::Churn => ("churning", "steady", None, "Most churn", None),
        ColorBy::Age => (
            "frozen",
            "stale",
            Some("active"),
            "Frozen code",
            Some("Active code"),
        ),
        ColorBy::Knowledge => (
            "at risk of knowledge loss",
            "shared",
            None,
            "Knowledge at risk",
            None,
        ),
        ColorBy::Cycles => ("in a cycle", "in no cycle", None, "Dependency cycles", None),
    };
    Words {
        risk,
        fine,
        good,
        risk_callout,
        good_callout,
    }
}

/// How a file is drawn.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Shade {
    pub heat: f64,
    pub label: String,
}

/// One entry of the legend: what a heat means.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Key {
    pub label: &'static str,
    pub heat: f64,
}

/// The shades by path relative to the analysed directory, and the legend.
pub type Shades = (HashMap<PathBuf, Shade>, Vec<Key>);

fn key(label: &'static str, heat: f64) -> Key {
    Key { label, heat }
}

/// Red below 70, green from 90, the deeper the further.
pub fn score_heat(score: f64) -> f64 {
    match score {
        s if s < 70.0 => -((70.0 - s) / 40.0).clamp(0.25, 1.0),
        s if s >= 90.0 => ((s - 85.0) / 15.0).clamp(0.3, 1.0),
        _ => 0.0,
    }
}

/// Green from `friendly`, red below `risk`, grey between.
pub fn llm_heat(score: f64, (friendly, risk): (f64, f64)) -> f64 {
    match score {
        s if s >= friendly => 1.0,
        s if s < risk => -1.0,
        _ => 0.0,
    }
}

/// Of the largest hotspot, the share a file reaches; under a tenth is fine.
pub fn hotspot_heat(score: usize, max: usize) -> f64 {
    let share = score as f64 / max.max(1) as f64;
    if share < 0.1 { 0.0 } else { -share }
}

pub fn churn_heat(level: ChurnLevel) -> f64 {
    match level {
        ChurnLevel::High => -1.0,
        ChurnLevel::Medium => -0.5,
        ChurnLevel::Low => 0.0,
    }
}

pub fn age_heat(status: AgeStatus) -> f64 {
    match status {
        AgeStatus::Active => 0.8,
        AgeStatus::Stale => 0.0,
        AgeStatus::Frozen => -0.35,
    }
}

pub fn risk_heat(risk: RiskLevel, knowledge_loss: bool) -> f64 {
    match risk {
        _ if knowledge_loss => -1.0,
        RiskLevel::Critical => -1.0,
        RiskLevel::High => -0.66,
        RiskLevel::Medium => -0.33,
        RiskLevel::Low => 0.0,
    }
}

/// The paths of the repository as paths under the analysed directory.
struct Git {
    repo: GitRepo,
    prefix: PathBuf,
}

impl Git {
    fn open(cfg: &WalkConfig<'_>) -> Result<Git, Box<dyn Error>> {
        let repo = GitRepo::open(cfg.path)
            .map_err(|e| format!("--color-by needs a git repository: {e}"))?;
        let (_, prefix) = repo.walk_prefix(cfg.path)?;
        Ok(Git { repo, prefix })
    }

    fn local(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(&self.prefix).ok().map(Path::to_path_buf)
    }
}

/// The shades of `color_by`, history read since `since` (a Unix time).
pub fn shades(
    cfg: &WalkConfig<'_>,
    color_by: ColorBy,
    since: Option<i64>,
    scores: &[FileScore],
) -> Result<Shades, Box<dyn Error>> {
    match color_by {
        ColorBy::Score => Ok(by_score(cfg.path, scores)),
        ColorBy::Hotspots => by_hotspots(cfg, since),
        ColorBy::Churn => by_churn(cfg, since),
        ColorBy::Age => by_age(cfg),
        ColorBy::Knowledge => by_knowledge(cfg, since),
        ColorBy::Cycles => Ok(by_cycles(cfg)),
        ColorBy::Llm => Ok(by_llm(
            cfg.path,
            scores,
            crate::config::KimunConfig::load().map.llm_thresholds(),
        )),
    }
}

fn by_score(root: &Path, scores: &[FileScore]) -> Shades {
    let shades = scores.iter().map(|f| {
        let label = format!("score {:.1} ({})", f.score, f.grade);
        let path = f.path.strip_prefix(root).unwrap_or(&f.path).to_path_buf();
        (
            path,
            Shade {
                heat: score_heat(f.score),
                label,
            },
        )
    });
    let legend = vec![
        key("below 70", -0.7),
        key("70 to 90", 0.0),
        key("90 and above", 0.7),
    ];
    (shades.collect(), legend)
}

fn by_llm(root: &Path, scores: &[FileScore], thresholds: (f64, f64)) -> Shades {
    let shades = scores.iter().map(|f| {
        let heat = llm_heat(f.score, thresholds);
        let class = match heat {
            h if h > 0.0 => "LLM-friendly",
            h if h < 0.0 => "an LLM risk",
            _ => "needs care",
        };
        let label = format!("score {:.1} ({}): {class}", f.score, f.grade);
        let path = f.path.strip_prefix(root).unwrap_or(&f.path).to_path_buf();
        (path, Shade { heat, label })
    });
    let legend = vec![
        key("LLM risk", -1.0),
        key("needs care", 0.0),
        key("LLM-friendly", 1.0),
    ];
    (shades.collect(), legend)
}

fn by_hotspots(cfg: &WalkConfig<'_>, since: Option<i64>) -> Result<Shades, Box<dyn Error>> {
    let kcfg = crate::config::KimunConfig::load();
    let metric = kcfg.hotspots.resolve_complexity(None);
    let git = Git::open(cfg)?;
    let found = crate::hotspots::collect(cfg, since, &metric)?.unwrap_or_default();
    let max = found.iter().map(|h| h.score).max().unwrap_or(0);
    let shades = found.iter().filter_map(|h| {
        let label = format!(
            "{} commits × {} {metric} = {}",
            h.commits, h.complexity, h.score
        );
        Some((
            git.local(&h.path)?,
            Shade {
                heat: hotspot_heat(h.score, max),
                label,
            },
        ))
    });
    let legend = vec![
        key("under a tenth of the top", 0.0),
        key("half of the top", -0.5),
        key("the top", -1.0),
    ];
    Ok((shades.collect(), legend))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn by_churn(cfg: &WalkConfig<'_>, since: Option<i64>) -> Result<Shades, Box<dyn Error>> {
    let git = Git::open(cfg)?;
    let now = now();
    let shades = git
        .repo
        .file_frequencies(since)?
        .into_iter()
        .filter_map(|f| {
            let window = (since.unwrap_or(f.first_commit), now);
            let churn = crate::churn::analyzer::classify(
                f.path.clone(),
                "",
                f.commits,
                (f.first_commit, f.last_commit),
                window,
            );
            let label = format!(
                "{} commits, {:.1} a month ({})",
                churn.commits,
                churn.rate,
                churn.level.label()
            );
            Some((
                git.local(&f.path)?,
                Shade {
                    heat: churn_heat(churn.level),
                    label,
                },
            ))
        });
    let legend = vec![key("low", 0.0), key("medium", -0.5), key("high", -1.0)];
    Ok((shades.collect(), legend))
}

fn by_age(cfg: &WalkConfig<'_>) -> Result<Shades, Box<dyn Error>> {
    let kcfg = crate::config::KimunConfig::load();
    let thresholds = AgeThresholds {
        active_days: kcfg.age.resolve_active_days(None),
        frozen_days: kcfg.age.resolve_frozen_days(None),
    };
    let git = Git::open(cfg)?;
    let (walk_root, _) = git.repo.walk_prefix(cfg.path)?;
    let files = crate::walk::code_files(&walk_root, cfg.exclude_tests(), cfg.filter);
    let targets: Vec<PathBuf> = files
        .iter()
        .map(|(f, _)| GitRepo::to_git_path(&walk_root, &git.prefix, f))
        .collect();
    let now = now();
    let modified = git.repo.last_modified_per_file(&targets)?;
    let shades = modified.into_iter().filter_map(|(path, when)| {
        let age = crate::age::analyzer::classify(path.clone(), "", when, now, &thresholds);
        let label = format!(
            "{} days since the last change ({})",
            age.age_days,
            age.status.label()
        );
        Some((
            git.local(&path)?,
            Shade {
                heat: age_heat(age.status),
                label,
            },
        ))
    });
    let legend = vec![key("active", 0.8), key("stale", 0.0), key("frozen", -0.35)];
    Ok((shades.collect(), legend))
}

fn by_knowledge(cfg: &WalkConfig<'_>, since: Option<i64>) -> Result<Shades, Box<dyn Error>> {
    let git = Git::open(cfg)?;
    let recent = since
        .map(|ts| git.repo.recent_authors(Some(ts)))
        .transpose()?;
    let (owned, _) = crate::knowledge::collect(cfg, &git.repo, recent.as_ref())?;
    let shades = owned.into_iter().filter_map(|o| {
        let gone = if o.knowledge_loss {
            ", owner inactive"
        } else {
            ""
        };
        let label = format!(
            "{} owns {:.0}% ({} risk{gone})",
            o.primary_owner,
            o.ownership_pct,
            o.risk.label()
        );
        Some((
            git.local(&o.path)?,
            Shade {
                heat: risk_heat(o.risk, o.knowledge_loss),
                label,
            },
        ))
    });
    let legend = vec![
        key("low", 0.0),
        key("medium", -0.33),
        key("high", -0.66),
        key("critical", -1.0),
    ];
    Ok((shades.collect(), legend))
}

fn by_cycles(cfg: &WalkConfig<'_>) -> Shades {
    let result = crate::deps::analyze(cfg);
    let shades = result.entries.into_iter().map(|e| {
        let (heat, label) = match e.in_cycle {
            true => (-1.0, "in a dependency cycle"),
            false => (0.0, "in no cycle"),
        };
        (
            e.path,
            Shade {
                heat,
                label: label.to_string(),
            },
        )
    });
    (
        shades.collect(),
        vec![key("in no cycle", 0.0), key("in a cycle", -1.0)],
    )
}

#[cfg(test)]
#[path = "metric_test.rs"]
mod tests;
