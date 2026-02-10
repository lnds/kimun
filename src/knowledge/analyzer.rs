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
mod tests {
    use super::*;

    fn make_blame(author: &str, email: &str, lines: usize, time: i64) -> BlameInfo {
        BlameInfo {
            author: author.to_string(),
            email: email.to_string(),
            lines,
            last_commit_time: time,
        }
    }

    #[test]
    fn test_critical_single_owner() {
        let blames = vec![make_blame("Alice", "alice@x.com", 90, 100)];
        let recent = HashSet::from(["alice@x.com".to_string()]);
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);

        assert_eq!(result.risk, RiskLevel::Critical);
        assert_eq!(result.primary_owner, "Alice");
        assert!((result.ownership_pct - 100.0).abs() < 0.01);
        assert!(!result.knowledge_loss);
    }

    #[test]
    fn test_high_risk() {
        let blames = vec![
            make_blame("Alice", "alice@x.com", 70, 100),
            make_blame("Bob", "bob@x.com", 30, 100),
        ];
        let recent = HashSet::new();
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert_eq!(result.risk, RiskLevel::High);
    }

    #[test]
    fn test_medium_risk() {
        let blames = vec![
            make_blame("Alice", "alice@x.com", 50, 100),
            make_blame("Bob", "bob@x.com", 40, 100),
            make_blame("Carol", "carol@x.com", 10, 100),
        ];
        let recent = HashSet::new();
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert_eq!(result.risk, RiskLevel::Medium);
    }

    #[test]
    fn test_low_risk() {
        let blames = vec![
            make_blame("Alice", "alice@x.com", 25, 100),
            make_blame("Bob", "bob@x.com", 25, 100),
            make_blame("Carol", "carol@x.com", 25, 100),
            make_blame("Dan", "dan@x.com", 25, 100),
        ];
        let recent = HashSet::new();
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert_eq!(result.risk, RiskLevel::Low);
    }

    #[test]
    fn test_knowledge_loss_detected() {
        let blames = vec![make_blame("Alice", "alice@x.com", 100, 100)];
        // Alice is NOT in recent authors → knowledge loss
        let recent = HashSet::from(["bob@x.com".to_string()]);
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert!(result.knowledge_loss);
    }

    #[test]
    fn test_no_knowledge_loss_when_no_since() {
        let blames = vec![make_blame("Alice", "alice@x.com", 100, 100)];
        // Empty recent_authors means --since was not used → no knowledge loss check
        let recent = HashSet::new();
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert!(!result.knowledge_loss);
    }

    #[test]
    fn test_empty_blames() {
        let blames: Vec<BlameInfo> = vec![];
        let recent = HashSet::new();
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.risk, RiskLevel::Low);
    }

    #[test]
    fn test_contributors_count() {
        let blames = vec![
            make_blame("Alice", "alice@x.com", 50, 100),
            make_blame("Bob", "bob@x.com", 50, 100),
        ];
        let recent = HashSet::new();
        let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
        assert_eq!(result.contributors, 2);
    }
}
