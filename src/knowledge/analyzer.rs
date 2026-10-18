/// Knowledge map analyzer — code ownership via git blame.
///
/// For each file, determines the primary owner, ownership concentration,
/// contributor count, and bus-factor risk level. Optionally detects
/// knowledge loss when the primary owner is no longer active.
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Serialize;

use crate::git::BlameInfo;

/// Bus-factor risk level based on ownership concentration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
}

impl RiskLevel {
    /// Human-readable label for display in reports.
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::High => "HIGH",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Low => "LOW",
        }
    }

    /// Numeric key for sorting risk levels (0 = most critical).
    pub fn sort_key(&self) -> u8 {
        match self {
            RiskLevel::Critical => 0,
            RiskLevel::High => 1,
            RiskLevel::Medium => 2,
            RiskLevel::Low => 3,
        }
    }
}

/// Internal representation of a single author's contribution to a file.
struct AuthorContribution {
    author: String,
    email: String,
    percentage: f64,
    active: bool,
}

/// Ownership analysis result for a single file.
pub struct FileOwnership {
    pub path: PathBuf,
    pub language: String,
    pub total_lines: usize,
    pub primary_owner: String,
    pub primary_email: String,
    pub ownership_pct: f64,
    pub contributors: usize,
    pub risk: RiskLevel,
    pub knowledge_loss: bool,
}

/// Compute ownership metrics from git blame data for a single file.
/// With `recent_authors` (the `--since` window), a file whose primary owner
/// is not among them is at risk of knowledge loss; with an empty window,
/// every owner is inactive.
pub fn compute_ownership(
    path: PathBuf,
    language: &str,
    blames: &[BlameInfo],
    recent_authors: Option<&HashSet<String>>,
) -> FileOwnership {
    let total_lines: usize = blames.iter().map(|b| b.lines).sum();

    if total_lines == 0 || blames.is_empty() {
        return FileOwnership {
            path,
            language: language.to_string(),
            total_lines: 0,
            primary_owner: "unknown".to_string(),
            primary_email: String::new(),
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
                email: b.email.clone(),
                percentage: pct,
                active: recent_authors.is_none_or(|r| r.contains(&b.identity())),
            }
        })
        .collect();

    let primary = &contributions[0]; // blames are sorted by lines desc
    let risk = classify_risk(&contributions);
    let knowledge_loss = !primary.active;

    FileOwnership {
        path,
        language: language.to_string(),
        total_lines,
        primary_owner: primary.author.clone(),
        primary_email: primary.email.clone(),
        ownership_pct: primary.percentage,
        contributors: contributions.len(),
        risk,
        knowledge_loss,
    }
}

/// Classify bus-factor risk based on ownership concentration.
/// Critical: single owner ≥80%. High: top owner ≥60%.
/// Medium: top 2-3 owners combine ≥80%. Low: otherwise.
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
    if top_combined >= 80.0 {
        return RiskLevel::Medium;
    }

    RiskLevel::Low
}

/// Aggregated ownership summary for a single author across all files they own.
pub struct AuthorSummary {
    pub author: String,
    /// Number of files where this author is the primary owner.
    pub files_owned: usize,
    /// Total lines across all owned files.
    pub total_lines: usize,
    /// Unique languages across owned files (sorted).
    pub languages: Vec<String>,
    /// Worst risk level among owned files.
    pub worst_risk: RiskLevel,
    /// Number of owned files flagged for knowledge loss (primary owner inactive).
    pub knowledge_loss_files: usize,
}

/// Aggregate per-file ownership data by primary owner.
/// Each entry in the result represents one author's total footprint.
pub fn aggregate_by_author(files: &[FileOwnership]) -> Vec<AuthorSummary> {
    use std::collections::{BTreeMap, BTreeSet};

    // key: identity → (name, files_owned, total_lines, languages, worst_risk, knowledge_loss_files)
    type Entry = (String, usize, usize, BTreeSet<String>, RiskLevel, usize);
    let mut map: BTreeMap<String, Entry> = BTreeMap::new();

    for f in files {
        let key = crate::git::identity(&f.primary_owner, &f.primary_email);
        let entry = map.entry(key).or_insert((
            f.primary_owner.clone(),
            0,
            0,
            BTreeSet::new(),
            RiskLevel::Low,
            0,
        ));
        entry.1 += 1;
        entry.2 += f.total_lines;
        entry.3.insert(f.language.clone());
        if f.risk.sort_key() < entry.4.sort_key() {
            entry.4 = f.risk;
        }
        if f.knowledge_loss {
            entry.5 += 1;
        }
    }

    map.into_values()
        .map(
            |(author, files_owned, total_lines, languages, worst_risk, knowledge_loss_files)| {
                AuthorSummary {
                    author,
                    files_owned,
                    total_lines,
                    languages: languages.into_iter().collect(),
                    worst_risk,
                    knowledge_loss_files,
                }
            },
        )
        .collect()
}

/// How many lines, against those of a file's main author, another author
/// needs to count as an author of the file too. Avelino et al. ("A novel
/// approach for estimating truck factors", ICPC 2016) use the same 0.75 on
/// a normalized degree of authorship; here the measure is blame lines.
pub const AUTHOR_SHARE: f64 = 0.75;

/// One contributor in the project-wide bus factor.
pub struct BusFactorEntry {
    pub author: String,
    /// Lines of the analyzed files that blame gives to this contributor.
    pub lines: usize,
    /// This contributor's share of those lines.
    pub pct: f64,
    /// Files this contributor is an author of (see [`AUTHOR_SHARE`]).
    pub files: usize,
    /// Share of lines in files left with no author once this contributor
    /// and everyone ranked above have left. Set only for those counted in
    /// the bus factor.
    pub orphaned_pct: Option<f64>,
    /// Whether this contributor is counted in the bus factor.
    pub is_critical: bool,
    /// Whether this contributor committed in the `--since` window; `None`
    /// when no window was given.
    pub active: Option<bool>,
}

/// Project-wide bus factor result.
pub struct BusFactor {
    /// How many contributors have to leave before more than `threshold`% of
    /// the code is in files with no author left. 0 when that is already so
    /// because of authors who stopped contributing.
    pub factor: usize,
    /// Share of the code, in lines, that has to be left without an author.
    pub threshold: f64,
    pub total_files: usize,
    /// Total blame lines across all analyzed files.
    pub total_lines: usize,
    /// Share of lines in files whose authors are all inactive, before anyone
    /// else leaves; `None` when no `--since` window was given.
    pub orphaned_by_inactivity: Option<f64>,
    /// Those counted in the bus factor, in the order they leave, then the
    /// rest by the lines of the files they are authors of.
    pub contributors: Vec<BusFactorEntry>,
}

/// What blame says about one person across the project.
#[derive(Default)]
struct Person {
    /// Lines under each name this person signed with.
    names: HashMap<String, usize>,
    lines: usize,
    files: usize,
    /// Lines of the files this person is an author of.
    authored_lines: usize,
}

impl Person {
    /// The name this person signed most lines with.
    fn name(&self) -> String {
        self.names
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(name, _)| name.clone())
            .unwrap_or_default()
    }
}

/// Who wrote what: each person, and each file with its lines and the
/// identities of its authors.
#[derive(Default)]
struct Authorship {
    people: HashMap<String, Person>,
    files: Vec<(usize, Vec<String>)>,
    total_lines: usize,
}

impl Authorship {
    fn read(files: &[Vec<BlameInfo>]) -> Self {
        let mut authorship = Self::default();
        for blames in files {
            authorship.add_file(blames);
        }
        authorship
    }

    fn add_file(&mut self, blames: &[BlameInfo]) {
        let size: usize = blames.iter().map(|b| b.lines).sum();
        let authors = file_authors(blames);
        for b in blames {
            let person = self.people.entry(b.identity()).or_default();
            *person.names.entry(b.author.clone()).or_default() += b.lines;
            person.lines += b.lines;
        }
        for key in &authors {
            let person = self.people.entry(key.clone()).or_default();
            person.files += 1;
            person.authored_lines += size;
        }
        self.files.push((size, authors));
        self.total_lines += size;
    }

    /// Share of lines in files whose authors are all in `gone`.
    fn orphaned_pct(&self, gone: &HashSet<&str>) -> f64 {
        let lines: usize = self
            .files
            .iter()
            .filter(|(_, authors)| authors.iter().all(|a| gone.contains(a.as_str())))
            .map(|(size, _)| size)
            .sum();
        lines as f64 / self.total_lines as f64 * 100.0
    }

    /// Everyone, the author of the most lines first.
    fn ranked(&self) -> Vec<&str> {
        let mut ranked: Vec<&str> = self.people.keys().map(String::as_str).collect();
        ranked.sort_by(|a, b| {
            let (pa, pb) = (&self.people[*a], &self.people[*b]);
            pb.authored_lines
                .cmp(&pa.authored_lines)
                .then_with(|| pb.lines.cmp(&pa.lines))
                .then_with(|| a.cmp(b))
        });
        ranked
    }

    fn entry(&self, key: &str, orphaned_pct: Option<f64>, active: Option<bool>) -> BusFactorEntry {
        let person = &self.people[key];
        BusFactorEntry {
            author: person.name(),
            lines: person.lines,
            pct: person.lines as f64 / self.total_lines as f64 * 100.0,
            files: person.files,
            orphaned_pct,
            is_critical: orphaned_pct.is_some(),
            active,
        }
    }
}

/// The identities of the authors of a file: those with at least
/// [`AUTHOR_SHARE`] of the lines of its main author.
fn file_authors(blames: &[BlameInfo]) -> Vec<String> {
    let top = blames.iter().map(|b| b.lines).max().unwrap_or(0);
    let mut authors: Vec<String> = Vec::new();
    if top == 0 {
        return authors;
    }
    for b in blames {
        let key = b.identity();
        if b.lines as f64 >= top as f64 * AUTHOR_SHARE && !authors.contains(&key) {
            authors.push(key);
        }
    }
    authors
}

/// Compute the bus factor (truck factor) from the blame of each file.
///
/// The authors of a file are those with at least [`AUTHOR_SHARE`] of the
/// lines of its main author. Contributors leave one at a time, the one who
/// is an author of the most lines first, until more than `threshold`% of
/// the lines are in files with no author left; the bus factor is how many
/// left. Files weigh by their lines, so a one-line file does not count as
/// much as the core of the project.
///
/// With `active`, the authors outside it have already left: the files they
/// alone wrote are orphaned before anyone is counted.
pub fn compute_bus_factor(
    files: &[Vec<BlameInfo>],
    active: Option<&HashSet<String>>,
    threshold: f64,
) -> BusFactor {
    let authorship = Authorship::read(files);
    if authorship.total_lines == 0 {
        return BusFactor {
            factor: 0,
            threshold,
            total_files: files.len(),
            total_lines: 0,
            orphaned_by_inactivity: None,
            contributors: vec![],
        };
    }

    let is_active = |key: &str| active.is_none_or(|a| a.contains(key));
    let ranked = authorship.ranked();
    let mut gone: HashSet<&str> = ranked.iter().copied().filter(|k| !is_active(k)).collect();
    let orphaned_by_inactivity = active.map(|_| authorship.orphaned_pct(&gone));

    let mut leaving: Vec<(&str, f64)> = Vec::new();
    let mut orphaned = authorship.orphaned_pct(&gone);
    for &key in ranked.iter().filter(|k| is_active(k)) {
        if orphaned > threshold {
            break;
        }
        gone.insert(key);
        orphaned = authorship.orphaned_pct(&gone);
        leaving.push((key, orphaned));
    }

    let activity = |key: &str| active.map(|_| is_active(key));
    let staying = ranked
        .iter()
        .filter(|k| !leaving.iter().any(|(l, _)| l == *k));
    let contributors = leaving
        .iter()
        .map(|&(key, pct)| authorship.entry(key, Some(pct), activity(key)))
        .chain(staying.map(|&key| authorship.entry(key, None, activity(key))))
        .collect();

    BusFactor {
        factor: leaving.len(),
        threshold,
        total_files: files.len(),
        total_lines: authorship.total_lines,
        orphaned_by_inactivity,
        contributors,
    }
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
