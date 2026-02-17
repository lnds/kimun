use std::collections::HashSet;
use std::path::PathBuf;

use serde::Serialize;

use crate::git::BlameInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
}

impl RiskLevel {
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::High => "HIGH",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Low => "LOW",
        }
    }

    pub fn sort_key(&self) -> u8 {
        match self {
            RiskLevel::Critical => 0,
            RiskLevel::High => 1,
            RiskLevel::Medium => 2,
            RiskLevel::Low => 3,
        }
    }
}

struct AuthorContribution {
    author: String,
    percentage: f64,
    active: bool,
}

pub struct FileOwnership {
    pub path: PathBuf,
    pub language: String,
    pub total_lines: usize,
    pub primary_owner: String,
    pub ownership_pct: f64,
    pub contributors: usize,
    pub risk: RiskLevel,
    pub knowledge_loss: bool,
}

pub fn compute_ownership(
    path: PathBuf,
    language: &str,
    blames: &[BlameInfo],
    recent_authors: &HashSet<String>,
) -> FileOwnership {
    let total_lines: usize = blames.iter().map(|b| b.lines).sum();

    if total_lines == 0 || blames.is_empty() {
        return FileOwnership {
            path,
            language: language.to_string(),
            total_lines: 0,
            primary_owner: "unknown".to_string(),
            ownership_pct: 0.0,
            contributors: 0,
            risk: RiskLevel::Low,
            knowledge_loss: false,
        };
    }

    let contributions: Vec<AuthorContribution> = blames
        .iter()
        .map(|b| {
            let pct = (b.lines as f64 / total_lines as f64) * 100.0;
            AuthorContribution {
                author: b.author.clone(),
                percentage: pct,
                active: recent_authors.contains(&b.email),
            }
        })
        .collect();

    let primary = &contributions[0]; // blames are sorted by lines desc
    let risk = classify_risk(&contributions);
    let knowledge_loss = !recent_authors.is_empty() && !primary.active;

    FileOwnership {
        path,
        language: language.to_string(),
        total_lines,
        primary_owner: primary.author.clone(),
        ownership_pct: primary.percentage,
        contributors: contributions.len(),
        risk,
        knowledge_loss,
    }
}

fn classify_risk(contributors: &[AuthorContribution]) -> RiskLevel {
    if contributors.is_empty() {
        return RiskLevel::Low;
    }

    let top_pct = contributors[0].percentage;

    if top_pct >= 80.0 {
        return RiskLevel::Critical;
    }
    if top_pct >= 60.0 {
        return RiskLevel::High;
    }

    // Check if top 2-3 contributors combine for >80%
    let top_combined: f64 = contributors.iter().take(3).map(|c| c.percentage).sum();
    if contributors.len() <= 3 && top_combined >= 80.0 {
        return RiskLevel::Medium;
    }

    RiskLevel::Low
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
