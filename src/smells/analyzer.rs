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
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LongFunction => "long_function",
            Self::LongParameterList => "long_params",
            Self::TodoDebt => "todo_debt",
            Self::MagicNumber => "magic_number",
            Self::CommentedOutCode => "commented_code",
        }
    }

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
    fn smell_kind_as_str_all_variants() {
        assert_eq!(SmellKind::LongFunction.as_str(), "long_function");
        assert_eq!(SmellKind::LongParameterList.as_str(), "long_params");
        assert_eq!(SmellKind::TodoDebt.as_str(), "todo_debt");
        assert_eq!(SmellKind::MagicNumber.as_str(), "magic_number");
        assert_eq!(SmellKind::CommentedOutCode.as_str(), "commented_code");
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
