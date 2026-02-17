use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Serialize;

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

/// Compute FNV-1a hash of a window of normalized code lines.
///
/// Uses a 0xFF separator between lines to prevent collisions where line
/// boundaries shift (e.g. `"ab"+"cd"` vs `"a"+"bcd"`). The hash is stable
/// and deterministic across runs, making it suitable for equality pre-checks.
fn hash_window(lines: &[NormalizedLine]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for line in lines {
        for byte in line.content.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }
        // Separator to avoid "ab"+"cd" colliding with "a"+"bcd"
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

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

/// Phase 2: filter candidate hashes to those with 2+ truly identical locations.
///
/// After dedup, verifies each hash group by comparing actual text content
/// (guards against FNV collisions). Skips patterns with more than
/// `MAX_OCCURRENCES` hits, which are likely boilerplate. Returns both a
/// reverse lookup (`LocationSet → hash`) for the extension phase, and the
/// sorted list of valid `(hash, locations)` entries.
fn validate_hashes(
    hash_map: HashMap<u64, LocationSet>,
    files: &[NormalizedFile],
    min_lines: usize,
    quiet: bool,
) -> (HashMap<LocationSet, u64>, Vec<(u64, LocationSet)>) {
    let mut location_to_hash: HashMap<LocationSet, u64> = HashMap::new();
    let mut valid_hashes: Vec<(u64, LocationSet)> = Vec::new();
    let mut skipped_common = 0usize;

    for (hash, mut locations) in hash_map {
        if locations.len() < 2 || locations.len() > MAX_OCCURRENCES {
            if locations.len() > MAX_OCCURRENCES {
                skipped_common += 1;
            }
            continue;
        }
        locations.sort();
        locations.dedup();
        if locations.len() < 2 {
            continue;
        }

        // Post-hash verification: confirm windows have identical text content
        let first = &locations[0];
        let first_window: Vec<&str> = files[first.0].lines[first.1..first.1 + min_lines]
            .iter()
            .map(|l| l.content.as_str())
            .collect();
        let all_match = locations[1..].iter().all(|(fi, off)| {
            files[*fi].lines[*off..*off + min_lines]
                .iter()
                .map(|l| l.content.as_str())
                .eq(first_window.iter().copied())
        });
        if !all_match {
            continue;
        }

        location_to_hash.insert(locations.clone(), hash);
        valid_hashes.push((hash, locations));
    }

    if skipped_common > 0 && !quiet {
        eprintln!(
            "note: skipped {skipped_common} patterns with >{MAX_OCCURRENCES} occurrences (likely boilerplate)"
        );
    }

    valid_hashes.sort_by(|a, b| a.1.cmp(&b.1));
    (location_to_hash, valid_hashes)
}

/// Extend a matched window backward by shifting all locations by -1 at each step.
///
/// Stops when any location reaches offset 0 or when the shifted location set
/// is not found in the lookup table. Marks consumed windows in `consumed` to
/// prevent re-processing. Returns the new start locations and backward count.
fn extend_backward(
    locations: &[(usize, usize)],
    location_to_hash: &HashMap<LocationSet, u64>,
    consumed: &mut HashSet<LocationSet>,
) -> (LocationSet, usize) {
    let mut start_locs = locations.to_vec();
    let mut backward_ext = 0usize;
    loop {
        if start_locs.iter().any(|(_, o)| *o == 0) {
            break;
        }
        let prev_locs: LocationSet = start_locs.iter().map(|(f, o)| (*f, o - 1)).collect();
        if location_to_hash.contains_key(&prev_locs) {
            consumed.insert(prev_locs.clone());
            start_locs = prev_locs;
            backward_ext += 1;
        } else {
            break;
        }
    }
    (start_locs, backward_ext)
}

/// Extend a matched window forward by shifting all locations by +1 at each step.
///
/// Stops when the shifted location set is not found in the lookup table.
/// Marks consumed windows in `consumed` to prevent re-processing.
/// Returns the number of forward extension steps taken.
fn extend_forward(
    locations: &[(usize, usize)],
    location_to_hash: &HashMap<LocationSet, u64>,
    consumed: &mut HashSet<LocationSet>,
) -> usize {
    let mut current_locs = locations.to_vec();
    let mut forward_ext = 0usize;
    loop {
        let next_locs: LocationSet = current_locs.iter().map(|(f, o)| (*f, o + 1)).collect();
        if location_to_hash.contains_key(&next_locs) {
            consumed.insert(next_locs.clone());
            current_locs = next_locs;
            forward_ext += 1;
        } else {
            break;
        }
    }
    forward_ext
}

/// Build a `DuplicateGroup` from the extended start locations and block size.
///
/// Maps each (file_index, offset) pair back to original line numbers via
/// `NormalizedLine::original_line_number`. Captures up to 5 sample lines
/// from the first location for display. Classifies severity as `Critical`
/// (3+ occurrences) or `Tolerable` (2 occurrences) per the Rule of Three.
fn build_group(
    files: &[NormalizedFile],
    start_locs: &[(usize, usize)],
    block_size: usize,
) -> DuplicateGroup {
    let mut dup_locations = Vec::new();
    let mut sample = Vec::new();

    for (file_idx, offset) in start_locs {
        let file = &files[*file_idx];
        let start_line = file.lines[*offset].original_line_number;
        let end_offset = (*offset + block_size - 1).min(file.lines.len() - 1);
        let end_line = file.lines[end_offset].original_line_number;

        if sample.is_empty() {
            let sample_end = (*offset + block_size).min(file.lines.len());
            sample = file.lines[*offset..sample_end]
                .iter()
                .take(5)
                .map(|l| l.content.clone())
                .collect();
        }

        dup_locations.push(DuplicateLocation {
            file_path: file.path.clone(),
            start_line,
            end_line,
        });
    }

    let severity = if dup_locations.len() >= 3 {
        DuplicationSeverity::Critical
    } else {
        DuplicationSeverity::Tolerable
    };

    DuplicateGroup {
        locations: dup_locations,
        line_count: block_size,
        sample,
        severity,
    }
}

/// Detect duplicate code blocks across files using a sliding window approach
/// with extension-based merging.
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
///
/// The `consumed` set tracks which windows have already been merged into a
/// larger block, preventing the same duplicate from being reported multiple
/// times. Invariant: once a window's location set is in `consumed`, it is
/// part of an already-emitted (or currently-being-built) `DuplicateGroup`.
pub fn detect_duplicates(
    files: &[NormalizedFile],
    min_lines: usize,
    quiet: bool,
) -> Vec<DuplicateGroup> {
    let hash_map = hash_all_windows(files, min_lines);
    let (location_to_hash, valid_hashes) = validate_hashes(hash_map, files, min_lines, quiet);

    // Phase 3: extension-based merging — see doc comment above for details.
    let mut consumed: HashSet<LocationSet> = HashSet::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    for (_hash, locations) in &valid_hashes {
        if consumed.contains(locations) {
            continue;
        }
        consumed.insert(locations.clone());

        let (start_locs, backward_ext) =
            extend_backward(locations, &location_to_hash, &mut consumed);
        let forward_ext = extend_forward(locations, &location_to_hash, &mut consumed);
        let block_size = min_lines + backward_ext + forward_ext;

        groups.push(build_group(files, &start_locs, block_size));
    }

    // Sort by severity (Critical first), then by duplicated lines descending
    groups.sort_by(|a, b| match a.severity.cmp(&b.severity) {
        std::cmp::Ordering::Equal => b.duplicated_lines().cmp(&a.duplicated_lines()),
        other => other,
    });
    groups
}

#[cfg(test)]
#[path = "detector_test.rs"]
mod tests;
