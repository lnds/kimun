/// Project-level configuration loaded from `.kimun.toml`.
///
/// Searched in order: git repository root, then current directory.
/// Missing file or parse errors are silently ignored — defaults apply.
///
/// Example `.kimun.toml`:
/// ```toml
/// [smells]
/// max_lines  = 30   # flag long functions at >30 body lines (default: 50)
/// max_params = 3    # flag functions with >3 parameters     (default: 4)
///
/// [dups]
/// min_lines      = 6    # minimum block size for duplication (default: 6)
/// max_duplicates = 10   # CI gate: fail if duplicate groups exceed N
/// max_dup_ratio  = 5.0  # CI gate: fail if duplicated-lines % exceeds this
///
/// [score]
/// model      = "cogcom"  # scoring model: cogcom or legacy   (default: cogcom)
/// fail_below = "B-"      # CI gate: fail if score is below this grade
///
/// [age]
/// active_days = 90    # days threshold for Active status  (default: 90)
/// frozen_days = 365   # days threshold for Frozen status  (default: 365)
///
/// [tc]
/// min_degree   = 3    # minimum commits per file to include (default: 3)
/// min_strength = 0.3  # minimum coupling strength to show
///
/// [hotspots]
/// complexity = "indent"  # complexity metric: indent, cycom, cogcom (default: indent)
/// ```
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct KimunConfig {
    #[serde(default)]
    pub smells: SmellsConfig,
    #[serde(default)]
    pub dups: DupsConfig,
    #[serde(default)]
    pub score: ScoreConfig,
    #[serde(default)]
    pub age: AgeConfig,
    #[serde(default)]
    pub tc: TcConfig,
    #[serde(default)]
    pub hotspots: HotspotsConfig,
}

/// Threshold overrides for `km smells`.
#[derive(Debug, Default, Deserialize)]
pub struct SmellsConfig {
    /// Maximum function body lines before flagging. CLI `--max-lines` takes precedence.
    pub max_lines: Option<usize>,
    /// Maximum parameter count before flagging. CLI `--max-params` takes precedence.
    pub max_params: Option<usize>,
}

impl SmellsConfig {
    pub const DEFAULT_MAX_LINES: usize = 50;
    pub const DEFAULT_MAX_PARAMS: usize = 4;

    /// Resolve final threshold: CLI flag > config file > hardcoded default.
    pub fn resolve_max_lines(&self, cli: Option<usize>) -> usize {
        cli.or(self.max_lines).unwrap_or(Self::DEFAULT_MAX_LINES)
    }

    pub fn resolve_max_params(&self, cli: Option<usize>) -> usize {
        cli.or(self.max_params).unwrap_or(Self::DEFAULT_MAX_PARAMS)
    }
}

/// Threshold overrides for `km dups` (also used by `km report` and `km score`).
#[derive(Debug, Default, Deserialize)]
pub struct DupsConfig {
    /// Minimum block size for a duplicate to count. CLI `--min-lines` takes precedence.
    pub min_lines: Option<usize>,
    /// CI gate: fail if duplicate groups exceed N. CLI `--max-duplicates` takes precedence.
    pub max_duplicates: Option<usize>,
    /// CI gate: fail if duplicated-lines ratio exceeds this %. CLI `--max-dup-ratio` takes precedence.
    pub max_dup_ratio: Option<f64>,
}

impl DupsConfig {
    pub const DEFAULT_MIN_LINES: usize = 6;

    pub fn resolve_min_lines(&self, cli: Option<usize>) -> usize {
        cli.or(self.min_lines).unwrap_or(Self::DEFAULT_MIN_LINES)
    }

    /// For optional gates: CLI value wins if set, otherwise config value (both may be None).
    pub fn resolve_max_duplicates(&self, cli: Option<usize>) -> Option<usize> {
        cli.or(self.max_duplicates)
    }

    pub fn resolve_max_dup_ratio(&self, cli: Option<f64>) -> Option<f64> {
        cli.or(self.max_dup_ratio)
    }
}

/// Configuration for `km score`.
#[derive(Debug, Default, Deserialize)]
pub struct ScoreConfig {
    /// Scoring model: `cogcom` (default) or `legacy`. CLI `--model` takes precedence.
    pub model: Option<String>,
    /// CI gate: fail if score is below this grade. CLI `--fail-below` takes precedence.
    pub fail_below: Option<String>,
}

impl ScoreConfig {
    pub const DEFAULT_MODEL: &'static str = "cogcom";

    pub fn resolve_model(&self, cli: Option<String>) -> String {
        cli.or_else(|| self.model.clone())
            .unwrap_or_else(|| Self::DEFAULT_MODEL.to_string())
    }

    pub fn resolve_fail_below(&self, cli: Option<String>) -> Option<String> {
        cli.or_else(|| self.fail_below.clone())
    }
}

/// Threshold overrides for `km age`.
#[derive(Debug, Default, Deserialize)]
pub struct AgeConfig {
    /// Days since last commit for a file to be Active. CLI `--active-days` takes precedence.
    pub active_days: Option<u64>,
    /// Days since last commit for a file to be Frozen. CLI `--frozen-days` takes precedence.
    pub frozen_days: Option<u64>,
}

impl AgeConfig {
    pub const DEFAULT_ACTIVE_DAYS: u64 = 90;
    pub const DEFAULT_FROZEN_DAYS: u64 = 365;

    pub fn resolve_active_days(&self, cli: Option<u64>) -> u64 {
        cli.or(self.active_days)
            .unwrap_or(Self::DEFAULT_ACTIVE_DAYS)
    }

    pub fn resolve_frozen_days(&self, cli: Option<u64>) -> u64 {
        cli.or(self.frozen_days)
            .unwrap_or(Self::DEFAULT_FROZEN_DAYS)
    }
}

/// Threshold overrides for `km tc`.
#[derive(Debug, Default, Deserialize)]
pub struct TcConfig {
    /// Minimum commits per file to be included. CLI `--min-degree` takes precedence.
    pub min_degree: Option<usize>,
    /// Minimum coupling strength to show. CLI `--min-strength` takes precedence.
    pub min_strength: Option<f64>,
}

impl TcConfig {
    pub const DEFAULT_MIN_DEGREE: usize = 3;

    pub fn resolve_min_degree(&self, cli: Option<usize>) -> usize {
        cli.or(self.min_degree).unwrap_or(Self::DEFAULT_MIN_DEGREE)
    }

    pub fn resolve_min_strength(&self, cli: Option<f64>) -> Option<f64> {
        cli.or(self.min_strength)
    }
}

/// Configuration for `km hotspots`.
#[derive(Debug, Default, Deserialize)]
pub struct HotspotsConfig {
    /// Complexity metric: `indent` (default), `cycom`, or `cogcom`.
    /// CLI `--complexity` takes precedence.
    pub complexity: Option<String>,
}

impl HotspotsConfig {
    pub const DEFAULT_COMPLEXITY: &'static str = "indent";

    pub fn resolve_complexity(&self, cli: Option<String>) -> String {
        cli.or_else(|| self.complexity.clone())
            .unwrap_or_else(|| Self::DEFAULT_COMPLEXITY.to_string())
    }
}

impl KimunConfig {
    /// Load `.kimun.toml` from the git root or current directory.
    /// Returns default config if no file is found or it cannot be parsed.
    pub fn load() -> Self {
        Self::try_load().unwrap_or_default()
    }

    fn try_load() -> Option<Self> {
        let path = Self::find()?;
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Prefer the git repository root; fall back to the current directory.
    fn find() -> Option<std::path::PathBuf> {
        if let Ok(repo) = git2::Repository::discover(".")
            && let Some(workdir) = repo.workdir()
        {
            let candidate = workdir.join(".kimun.toml");
            if candidate.exists() {
                return Some(candidate);
            }
        }
        let candidate = std::path::PathBuf::from(".kimun.toml");
        candidate.exists().then_some(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> KimunConfig {
        toml::from_str(toml).expect("valid toml")
    }

    // ── defaults ───────────────────────────────────────────────���────────────

    #[test]
    fn defaults_apply_when_no_config() {
        let cfg = KimunConfig::default();
        assert_eq!(
            cfg.smells.resolve_max_lines(None),
            SmellsConfig::DEFAULT_MAX_LINES
        );
        assert_eq!(
            cfg.smells.resolve_max_params(None),
            SmellsConfig::DEFAULT_MAX_PARAMS
        );
        assert_eq!(
            cfg.dups.resolve_min_lines(None),
            DupsConfig::DEFAULT_MIN_LINES
        );
        assert_eq!(cfg.score.resolve_model(None), ScoreConfig::DEFAULT_MODEL);
        assert_eq!(
            cfg.age.resolve_active_days(None),
            AgeConfig::DEFAULT_ACTIVE_DAYS
        );
        assert_eq!(
            cfg.age.resolve_frozen_days(None),
            AgeConfig::DEFAULT_FROZEN_DAYS
        );
        assert_eq!(
            cfg.tc.resolve_min_degree(None),
            TcConfig::DEFAULT_MIN_DEGREE
        );
        assert_eq!(
            cfg.hotspots.resolve_complexity(None),
            HotspotsConfig::DEFAULT_COMPLEXITY
        );
    }

    #[test]
    fn optional_gates_are_none_by_default() {
        let cfg = KimunConfig::default();
        assert!(cfg.dups.resolve_max_duplicates(None).is_none());
        assert!(cfg.dups.resolve_max_dup_ratio(None).is_none());
        assert!(cfg.score.resolve_fail_below(None).is_none());
        assert!(cfg.tc.resolve_min_strength(None).is_none());
    }

    // ── config file values ────────────────────────────────────────────���──────

    #[test]
    fn smells_config_is_parsed() {
        let cfg = parse("[smells]\nmax_lines = 30\nmax_params = 3\n");
        assert_eq!(cfg.smells.resolve_max_lines(None), 30);
        assert_eq!(cfg.smells.resolve_max_params(None), 3);
    }

    #[test]
    fn dups_config_is_parsed() {
        let cfg = parse("[dups]\nmin_lines = 8\nmax_duplicates = 10\nmax_dup_ratio = 5.0\n");
        assert_eq!(cfg.dups.resolve_min_lines(None), 8);
        assert_eq!(cfg.dups.resolve_max_duplicates(None), Some(10));
        assert_eq!(cfg.dups.resolve_max_dup_ratio(None), Some(5.0));
    }

    #[test]
    fn score_config_is_parsed() {
        let cfg = parse("[score]\nmodel = \"legacy\"\nfail_below = \"B-\"\n");
        assert_eq!(cfg.score.resolve_model(None), "legacy");
        assert_eq!(cfg.score.resolve_fail_below(None).as_deref(), Some("B-"));
    }

    #[test]
    fn age_config_is_parsed() {
        let cfg = parse("[age]\nactive_days = 60\nfrozen_days = 180\n");
        assert_eq!(cfg.age.resolve_active_days(None), 60);
        assert_eq!(cfg.age.resolve_frozen_days(None), 180);
    }

    #[test]
    fn tc_config_is_parsed() {
        let cfg = parse("[tc]\nmin_degree = 5\nmin_strength = 0.5\n");
        assert_eq!(cfg.tc.resolve_min_degree(None), 5);
        assert_eq!(cfg.tc.resolve_min_strength(None), Some(0.5));
    }

    #[test]
    fn hotspots_config_is_parsed() {
        let cfg = parse("[hotspots]\ncomplexity = \"cogcom\"\n");
        assert_eq!(cfg.hotspots.resolve_complexity(None), "cogcom");
    }

    // ── CLI overrides config ─────────────────────────────────────────────────

    #[test]
    fn cli_overrides_smells_config() {
        let cfg = parse("[smells]\nmax_lines = 30\nmax_params = 3\n");
        assert_eq!(cfg.smells.resolve_max_lines(Some(100)), 100);
        assert_eq!(cfg.smells.resolve_max_params(Some(10)), 10);
    }

    #[test]
    fn cli_overrides_dups_config() {
        let cfg = parse("[dups]\nmin_lines = 8\nmax_duplicates = 10\nmax_dup_ratio = 5.0\n");
        assert_eq!(cfg.dups.resolve_min_lines(Some(4)), 4);
        assert_eq!(cfg.dups.resolve_max_duplicates(Some(99)), Some(99));
        assert_eq!(cfg.dups.resolve_max_dup_ratio(Some(1.0)), Some(1.0));
    }

    #[test]
    fn cli_overrides_score_config() {
        let cfg = parse("[score]\nmodel = \"legacy\"\nfail_below = \"B-\"\n");
        assert_eq!(cfg.score.resolve_model(Some("cogcom".into())), "cogcom");
        assert_eq!(
            cfg.score.resolve_fail_below(Some("A".into())).as_deref(),
            Some("A")
        );
    }

    #[test]
    fn cli_overrides_age_config() {
        let cfg = parse("[age]\nactive_days = 60\nfrozen_days = 180\n");
        assert_eq!(cfg.age.resolve_active_days(Some(30)), 30);
        assert_eq!(cfg.age.resolve_frozen_days(Some(730)), 730);
    }

    #[test]
    fn cli_overrides_tc_config() {
        let cfg = parse("[tc]\nmin_degree = 5\nmin_strength = 0.5\n");
        assert_eq!(cfg.tc.resolve_min_degree(Some(1)), 1);
        assert_eq!(cfg.tc.resolve_min_strength(Some(0.9)), Some(0.9));
    }

    #[test]
    fn cli_overrides_hotspots_config() {
        let cfg = parse("[hotspots]\ncomplexity = \"cogcom\"\n");
        assert_eq!(
            cfg.hotspots.resolve_complexity(Some("cycom".into())),
            "cycom"
        );
    }

    // ── partial config (missing fields use defaults) ─────────────────────────

    #[test]
    fn partial_smells_config_uses_defaults_for_missing_fields() {
        let cfg = parse("[smells]\nmax_lines = 20\n");
        assert_eq!(cfg.smells.resolve_max_lines(None), 20);
        assert_eq!(
            cfg.smells.resolve_max_params(None),
            SmellsConfig::DEFAULT_MAX_PARAMS
        );
    }

    #[test]
    fn empty_config_uses_all_defaults() {
        let cfg = parse("");
        assert_eq!(
            cfg.smells.resolve_max_lines(None),
            SmellsConfig::DEFAULT_MAX_LINES
        );
        assert_eq!(
            cfg.dups.resolve_min_lines(None),
            DupsConfig::DEFAULT_MIN_LINES
        );
        assert_eq!(cfg.score.resolve_model(None), ScoreConfig::DEFAULT_MODEL);
        assert!(cfg.score.resolve_fail_below(None).is_none());
    }

    // ── file loading ─────────────────────────────────────────────────────────

    #[test]
    fn load_from_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".kimun.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "[smells]\nmax_lines = 25\nmax_params = 2").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let cfg: KimunConfig = toml::from_str(&content).unwrap();
        assert_eq!(cfg.smells.resolve_max_lines(None), 25);
        assert_eq!(cfg.smells.resolve_max_params(None), 2);
    }

    #[test]
    fn invalid_toml_falls_back_to_defaults() {
        let result: Result<KimunConfig, _> = toml::from_str("not valid toml ][");
        assert!(result.is_err());
        // load() swallows errors and returns Default
        let cfg = KimunConfig::default();
        assert_eq!(
            cfg.smells.resolve_max_lines(None),
            SmellsConfig::DEFAULT_MAX_LINES
        );
    }
}
