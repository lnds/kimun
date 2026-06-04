//! Types and orchestration for code smell detection.

use serde::Serialize;

use crate::detection::FunctionDetectionMarkers;
use crate::loc::counter::LineKind;

use super::rules;

/// Categories of code smells detected by the analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SmellKind {
    LongFunction,
    LongParameterList,
    TodoDebt,
    MagicNumber,
    CommentedOutCode,
}

impl SmellKind {
    /// Human-readable title for GitHub annotation titles.
    pub fn title(self) -> &'static str {
        match self {
            Self::LongFunction => "Long Function",
            Self::LongParameterList => "Long Parameter List",
            Self::TodoDebt => "TODO/FIXME Debt",
            Self::MagicNumber => "Magic Number",
            Self::CommentedOutCode => "Commented-Out Code",
        }
    }

    /// Short column header used in the per-file breakdown table.
    pub fn short_label(self) -> &'static str {
        match self {
            Self::LongFunction => "long",
            Self::LongParameterList => "param",
            Self::TodoDebt => "todo",
            Self::MagicNumber => "magic",
            Self::CommentedOutCode => "comm",
        }
    }

    /// All smell kinds in a fixed canonical order (used for stable table columns).
    pub const fn all() -> [SmellKind; 5] {
        [
            Self::MagicNumber,
            Self::LongFunction,
            Self::LongParameterList,
            Self::TodoDebt,
            Self::CommentedOutCode,
        ]
    }
}

/// A single detected smell instance at a specific line.
#[derive(Debug, Clone)]
pub struct SmellInstance {
    pub kind: SmellKind,
    pub line: usize,
    pub detail: String,
}

/// All smells detected in a single file.
pub struct FileSmells {
    pub smells: Vec<SmellInstance>,
}

/// Detect all smells in a file given its classified lines.
///
/// Returns `None` if no smells are found.
pub fn detect_smells(
    lines: &[String],
    kinds: &[LineKind],
    markers: &dyn FunctionDetectionMarkers,
    max_lines: usize,
    max_params: usize,
) -> Option<FileSmells> {
    let mut smells = Vec::new();

    smells.extend(rules::detect_long_functions(
        lines, kinds, markers, max_lines,
    ));
    smells.extend(rules::detect_long_params(lines, kinds, markers, max_params));
    smells.extend(rules::detect_todo_debt(lines, kinds));
    smells.extend(rules::detect_magic_numbers(
        lines,
        kinds,
        markers.line_comments(),
    ));
    smells.extend(rules::detect_commented_out_code(lines, kinds));

    if smells.is_empty() {
        return None;
    }

    smells.sort_by_key(|s| s.line);
    Some(FileSmells { smells })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smell_kind_short_label_all_variants() {
        assert_eq!(SmellKind::LongFunction.short_label(), "long");
        assert_eq!(SmellKind::LongParameterList.short_label(), "param");
        assert_eq!(SmellKind::TodoDebt.short_label(), "todo");
        assert_eq!(SmellKind::MagicNumber.short_label(), "magic");
        assert_eq!(SmellKind::CommentedOutCode.short_label(), "comm");
    }

    #[test]
    fn smell_kind_all_is_canonical_order() {
        assert_eq!(
            SmellKind::all(),
            [
                SmellKind::MagicNumber,
                SmellKind::LongFunction,
                SmellKind::LongParameterList,
                SmellKind::TodoDebt,
                SmellKind::CommentedOutCode,
            ]
        );
    }

    #[test]
    fn smell_kind_title_all_variants() {
        assert_eq!(SmellKind::LongFunction.title(), "Long Function");
        assert_eq!(SmellKind::LongParameterList.title(), "Long Parameter List");
        assert_eq!(SmellKind::TodoDebt.title(), "TODO/FIXME Debt");
        assert_eq!(SmellKind::MagicNumber.title(), "Magic Number");
        assert_eq!(SmellKind::CommentedOutCode.title(), "Commented-Out Code");
    }
}
