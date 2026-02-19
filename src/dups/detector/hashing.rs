/// FNV-1a hashing functions for duplicate code detection.
///
/// Uses the 64-bit FNV-1a algorithm for fast, deterministic hashing of
/// code windows and location sets. The hash is used as a pre-filter —
/// actual text comparison follows to guard against collisions.
use super::NormalizedLine;

/// Hash a LocationSet into a u64 using FNV-1a for efficient HashMap/HashSet keys.
/// Avoids O(n) hashing of the full Vec on every lookup.
pub fn hash_location_set(locs: &[(usize, usize)]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for (f, o) in locs {
        hash ^= *f as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= *o as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Compute FNV-1a hash of a window of normalized code lines.
///
/// Uses a 0xFF separator between lines to prevent collisions where line
/// boundaries shift (e.g. `"ab"+"cd"` vs `"a"+"bcd"`). The hash is stable
/// and deterministic across runs, making it suitable for equality pre-checks.
pub fn hash_window(lines: &[NormalizedLine]) -> u64 {
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
