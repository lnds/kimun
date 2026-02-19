/// Phase 3 helpers: extend matched windows backward and forward.
///
/// A sliding window finds the *minimum* duplicate block, but the actual
/// duplicated region may be larger. These functions grow the block by
/// checking whether adjacent shifted windows also exist in the lookup.
use std::collections::{HashMap, HashSet};

use super::hashing::hash_location_set;
use super::{LocationSet, NormalizedFile};

/// Extend a matched window backward by shifting all locations by -1 at each step.
///
/// Stops when any location reaches offset 0 or when the shifted location set
/// is not found in the lookup table. Marks consumed windows in `consumed` to
/// prevent re-processing. Returns the new start locations and backward count.
pub fn extend_backward(
    locations: &[(usize, usize)],
    location_to_hash: &HashMap<u64, u64>,
    consumed: &mut HashSet<u64>,
) -> (LocationSet, usize) {
    let mut start_locs = locations.to_vec();
    let mut backward_ext = 0usize;
    loop {
        if start_locs.iter().any(|(_, o)| *o == 0) {
            break;
        }
        let prev_locs: LocationSet = start_locs.iter().map(|(f, o)| (*f, o - 1)).collect();
        let prev_key = hash_location_set(&prev_locs);
        if location_to_hash.contains_key(&prev_key) {
            consumed.insert(prev_key);
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
pub fn extend_forward(
    locations: &[(usize, usize)],
    location_to_hash: &HashMap<u64, u64>,
    consumed: &mut HashSet<u64>,
) -> usize {
    let mut current_locs = locations.to_vec();
    let mut forward_ext = 0usize;
    loop {
        let next_locs: LocationSet = current_locs.iter().map(|(f, o)| (*f, o + 1)).collect();
        let next_key = hash_location_set(&next_locs);
        if location_to_hash.contains_key(&next_key) {
            consumed.insert(next_key);
            current_locs = next_locs;
            forward_ext += 1;
        } else {
            break;
        }
    }
    forward_ext
}

/// Verify that the extended block has identical text content at all locations.
///
/// This guards against hash collisions in `hash_location_set` during extension.
/// If a collision caused a false extension, we shrink the block back to the
/// largest verified size (at least `min_lines` from the original match).
pub fn verify_extended_block(
    files: &[NormalizedFile],
    start_locs: &[(usize, usize)],
    block_size: usize,
) -> usize {
    if start_locs.len() < 2 {
        return block_size;
    }
    let (first_fi, first_off) = start_locs[0];
    let first_file = &files[first_fi];
    let first_end = (first_off + block_size).min(first_file.lines.len());
    let first_lines = &first_file.lines[first_off..first_end];

    // Check each other location line-by-line; return the min matching length.
    let mut verified = first_lines.len();
    for &(fi, off) in &start_locs[1..] {
        let other_file = &files[fi];
        let other_end = (off + block_size).min(other_file.lines.len());
        let other_lines = &other_file.lines[off..other_end];
        let matching = first_lines
            .iter()
            .zip(other_lines.iter())
            .take_while(|(a, b)| a.content == b.content)
            .count();
        verified = verified.min(matching);
    }
    verified.max(1)
}
