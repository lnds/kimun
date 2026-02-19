/// Duplicate code detection using sliding window hashing with extension.
///
/// ## Algorithm
///
/// **Phase 1 — Hashing:** Slide a window of `min_lines` over every file,
/// hashing each window with FNV-1a. This produces a map from hash to the
/// list of (file, offset) locations where that window appears.
///
/// **Phase 2 — Validation:** Keep only hashes that appear at 2+ distinct
/// locations (after dedup). Verify that matching hashes have truly identical
/// text content (guards against FNV collisions). Also builds a reverse
/// lookup from `LocationSet → hash` used by Phase 3.
///
/// **Phase 3 — Extension-based merging:** A sliding window finds the
/// *minimum* duplicate block, but the actual duplicated region may be
/// larger. For each location set, we extend backward and forward by
/// checking whether adjacent shifted windows also exist in the lookup.
mod extension;
mod groups;
mod hashing;
mod validation;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use extension::{extend_backward, extend_forward, verify_extended_block};
use groups::build_group;
use hashing::{hash_location_set, hash_window};
use serde::Serialize;
use validation::validate_hashes;

/// Severity classification based on the Rule of Three.
/// - `Critical`: 3+ occurrences — should be refactored.
/// - `Tolerable`: 2 occurrences — acceptable duplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum DuplicationSeverity {
    Critical,
    Tolerable,
}

/// A normalized code line with its original position and content.
pub struct NormalizedLine {
    pub original_line_number: usize, // 1-based
    pub content: String,             // trimmed code text
}

/// A file's normalized code lines ready for duplicate detection.
pub struct NormalizedFile {
    pub path: PathBuf,
    pub lines: Vec<NormalizedLine>,
}

/// A location where a duplicate block appears.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateLocation {
    pub file_path: PathBuf,
    pub start_line: usize, // 1-based original line number
    pub end_line: usize,   // 1-based original line number (inclusive)
}

/// A group of identical code blocks found in different locations.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub locations: Vec<DuplicateLocation>,
    pub line_count: usize,
    pub sample: Vec<String>,
    pub severity: DuplicationSeverity,
}

impl DuplicateGroup {
    /// Lines duplicated beyond the first occurrence.
    pub fn duplicated_lines(&self) -> usize {
        self.line_count * (self.locations.len() - 1)
    }
}

/// Maximum occurrences before a pattern is considered boilerplate and skipped.
const MAX_OCCURRENCES: usize = 100;

/// A set of (file_index, line_offset) pairs identifying where a window appears.
type LocationSet = Vec<(usize, usize)>;

/// Phase 1: slide a window of `min_lines` over every file, hashing each window.
///
/// Returns a map from hash value to the list of (file_index, line_offset) pairs
/// where that hash occurs. Files shorter than `min_lines` are skipped entirely.
fn hash_all_windows(files: &[NormalizedFile], min_lines: usize) -> HashMap<u64, LocationSet> {
    let mut hash_map: HashMap<u64, LocationSet> = HashMap::new();
    for (file_idx, file) in files.iter().enumerate() {
        if file.lines.len() < min_lines {
            continue;
        }
        for offset in 0..=(file.lines.len() - min_lines) {
            let hash = hash_window(&file.lines[offset..offset + min_lines]);
            hash_map.entry(hash).or_default().push((file_idx, offset));
        }
    }
    hash_map
}

/// Detect duplicate code blocks across files using a sliding window approach
/// with extension-based merging. See module-level documentation for details.
pub fn detect_duplicates(
    files: &[NormalizedFile],
    min_lines: usize,
    quiet: bool,
) -> Vec<DuplicateGroup> {
    let hash_map = hash_all_windows(files, min_lines);
    let (location_to_hash, valid_hashes) = validate_hashes(hash_map, files, min_lines, quiet);

    // Phase 3: extension-based merging
    let mut consumed: HashSet<u64> = HashSet::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    for (_hash, locations) in &valid_hashes {
        let loc_key = hash_location_set(locations);
        if consumed.contains(&loc_key) {
            continue;
        }
        consumed.insert(loc_key);

        let (start_locs, backward_ext) =
            extend_backward(locations, &location_to_hash, &mut consumed);
        let forward_ext = extend_forward(locations, &location_to_hash, &mut consumed);
        let block_size = min_lines + backward_ext + forward_ext;

        // Post-extension content verification: confirm the extended block
        // is truly identical at all locations (guards against hash collisions
        // in location_to_hash that could produce false extensions).
        let verified_size = verify_extended_block(files, &start_locs, block_size);
        groups.push(build_group(files, &start_locs, verified_size));
    }

    // Sort by severity (Critical first), then by duplicated lines descending
    groups.sort_by(|a, b| match a.severity.cmp(&b.severity) {
        std::cmp::Ordering::Equal => b.duplicated_lines().cmp(&a.duplicated_lines()),
        other => other,
    });
    groups
}

#[cfg(test)]
#[path = "../detector_test.rs"]
mod tests;
