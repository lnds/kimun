/// Phase 2: validate candidate hash groups by verifying text equality.
///
/// Filters out FNV collisions and boilerplate patterns (>100 occurrences).
use std::collections::HashMap;

use super::{LocationSet, MAX_OCCURRENCES, NormalizedFile, without_overlaps};

/// Filter candidate hashes to those with 2+ truly identical locations.
///
/// After dedup, drops the locations that overlap an earlier one in the same
/// file (a run of repeated lines matches itself one line apart), then
/// verifies each hash group by comparing actual text content (guards
/// against FNV collisions). Skips patterns with more than `MAX_OCCURRENCES`
/// hits, which are likely boilerplate. Returns the sorted list of valid
/// `(hash, locations)` entries.
pub fn validate_hashes(
    hash_map: &HashMap<u64, LocationSet>,
    files: &[NormalizedFile],
    min_lines: usize,
    quiet: bool,
) -> Vec<(u64, LocationSet)> {
    let mut valid_hashes: Vec<(u64, LocationSet)> = Vec::new();
    let mut skipped_common = 0usize;

    for (&hash, locations) in hash_map {
        if locations.len() < 2 || locations.len() > MAX_OCCURRENCES {
            if locations.len() > MAX_OCCURRENCES {
                skipped_common += 1;
            }
            continue;
        }
        let mut locations = locations.clone();
        locations.sort();
        locations.dedup();
        let locations = without_overlaps(locations, min_lines);
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

        valid_hashes.push((hash, locations));
    }

    if skipped_common > 0 && !quiet {
        eprintln!(
            "note: skipped {skipped_common} patterns with >{MAX_OCCURRENCES} occurrences (likely boilerplate)"
        );
    }

    valid_hashes.sort_by_key(|x| x.1.clone());
    valid_hashes
}
