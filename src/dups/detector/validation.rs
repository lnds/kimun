/// Phase 2: validate candidate hash groups by verifying text equality.
///
/// Filters out FNV collisions and boilerplate patterns (>100 occurrences).
/// Builds a reverse lookup from LocationSet hash to window hash, used
/// by the extension phase to merge adjacent duplicate windows.
use std::collections::HashMap;

use super::hashing::hash_location_set;
use super::{LocationSet, MAX_OCCURRENCES, NormalizedFile};

/// Filter candidate hashes to those with 2+ truly identical locations.
///
/// After dedup, verifies each hash group by comparing actual text content
/// (guards against FNV collisions). Skips patterns with more than
/// `MAX_OCCURRENCES` hits, which are likely boilerplate. Returns both a
/// reverse lookup (`LocationSet → hash`) for the extension phase, and the
/// sorted list of valid `(hash, locations)` entries.
pub fn validate_hashes(
    hash_map: HashMap<u64, LocationSet>,
    files: &[NormalizedFile],
    min_lines: usize,
    quiet: bool,
) -> (HashMap<u64, u64>, Vec<(u64, LocationSet)>) {
    let mut location_to_hash: HashMap<u64, u64> = HashMap::new();
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

        location_to_hash.insert(hash_location_set(&locations), hash);
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
