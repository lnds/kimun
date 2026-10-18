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
/// larger. For each location set, we extend backward and forward while the
/// shifted windows at every location still hold the same text, whatever
/// else holds it.
///
/// The occurrences of a group never overlap within a file, and the lines a
/// project duplicates are the union over all groups
/// ([`duplicated_line_count`]): a line in two groups counts once.
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
    /// Where each occurrence starts, as (file index, normalized line), in
    /// the files the group was found in.
    #[serde(skip)]
    pub starts: Vec<(usize, usize)>,
}

impl DuplicateGroup {
    /// Lines duplicated beyond the first occurrence.
    pub fn duplicated_lines(&self) -> usize {
        self.line_count * (self.locations.len() - 1)
    }
}

/// The code lines duplicated in a project: those of every occurrence but
/// the first of each group, each line once however many groups hold it.
pub fn duplicated_line_count(groups: &[DuplicateGroup]) -> usize {
    let mut lines: HashSet<(usize, usize)> = HashSet::new();
    for group in groups {
        if group.starts.len() != group.locations.len() {
            // Built without its starts: no line to share with another group.
            return groups.iter().map(DuplicateGroup::duplicated_lines).sum();
        }
        for &(file, start) in group.starts.iter().skip(1) {
            lines.extend((start..start + group.line_count).map(|line| (file, line)));
        }
    }
    lines.len()
}

/// `locations`, sorted, without those that overlap an earlier kept one in
/// the same file: a block of `size` lines cannot start inside another.
pub(crate) fn without_overlaps(locations: LocationSet, size: usize) -> LocationSet {
    let mut kept: LocationSet = Vec::with_capacity(locations.len());
    for loc in locations {
        let overlaps = kept
            .last()
            .is_some_and(|&(file, start)| file == loc.0 && loc.1 < start + size);
        if !overlaps {
            kept.push(loc);
        }
    }
    kept
}

/// The hash of the window that starts at each (file, offset).
fn window_hashes(hash_map: &HashMap<u64, LocationSet>) -> HashMap<(usize, usize), u64> {
    hash_map
        .iter()
        .flat_map(|(hash, locs)| locs.iter().map(move |loc| (*loc, *hash)))
        .collect()
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
    let window_hash = window_hashes(&hash_map);
    let valid_hashes = validate_hashes(&hash_map, files, min_lines, quiet);

    // Phase 3: extension-based merging
    let mut consumed: HashSet<u64> = HashSet::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    for (_hash, locations) in &valid_hashes {
        let loc_key = hash_location_set(locations);
        if consumed.contains(&loc_key) {
            continue;
        }
        consumed.insert(loc_key);

        let (start_locs, backward_ext) = extend_backward(locations, &window_hash, &mut consumed);
        let forward_ext = extend_forward(locations, &window_hash, &mut consumed);
        let block_size = min_lines + backward_ext + forward_ext;

        // Post-extension content verification: confirm the extended block
        // is truly identical at all locations (guards against hash
        // collisions that could produce false extensions). A block that grew
        // into the next occurrence in the same file keeps that one out.
        let verified_size = verify_extended_block(files, &start_locs, block_size);
        let start_locs = without_overlaps(start_locs, verified_size);
        if start_locs.len() >= 2 {
            groups.push(build_group(files, &start_locs, verified_size));
        }
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
