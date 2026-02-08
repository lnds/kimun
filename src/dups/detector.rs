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

/// FNV-1a hash — stable, deterministic, fast.
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

/// Detect duplicate code blocks across files using a sliding window approach
/// with extension-based merging.
pub fn detect_duplicates(
    files: &[NormalizedFile],
    min_lines: usize,
    quiet: bool,
) -> Vec<DuplicateGroup> {
    // Phase 1: hash all windows of size min_lines
    let mut hash_map: HashMap<u64, Vec<(usize, usize)>> = HashMap::new();

    for (file_idx, file) in files.iter().enumerate() {
        if file.lines.len() < min_lines {
            continue;
        }
        for offset in 0..=(file.lines.len() - min_lines) {
            let hash = hash_window(&file.lines[offset..offset + min_lines]);
            hash_map.entry(hash).or_default().push((file_idx, offset));
        }
    }

    // Phase 2: build a lookup from sorted-location-set to hash
    // Filter to hashes with 2+ locations, skip overly common patterns
    let mut location_to_hash: HashMap<Vec<(usize, usize)>, u64> = HashMap::new();
    let mut valid_hashes: Vec<(u64, Vec<(usize, usize)>)> = Vec::new();
    let mut skipped_common = 0usize;

    for (hash, mut locations) in hash_map {
        if locations.len() < 2 {
            continue;
        }
        if locations.len() > MAX_OCCURRENCES {
            skipped_common += 1;
            continue;
        }
        locations.sort();
        locations.dedup();
        if locations.len() < 2 {
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

    // Phase 3: extension-based merging
    // For each location set, extend backward and forward to find maximal blocks
    let mut consumed: HashSet<Vec<(usize, usize)>> = HashSet::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    // Sort by first location for deterministic ordering
    valid_hashes.sort_by(|a, b| a.1.cmp(&b.1));

    for (_hash, locations) in &valid_hashes {
        if consumed.contains(locations) {
            continue;
        }
        consumed.insert(locations.clone());

        // Extend backward
        let mut start_locs = locations.clone();
        let mut backward_ext = 0usize;
        loop {
            if start_locs.iter().any(|(_, o)| *o == 0) {
                break;
            }
            let prev_locs: Vec<(usize, usize)> =
                start_locs.iter().map(|(f, o)| (*f, o - 1)).collect();
            if location_to_hash.contains_key(&prev_locs) {
                consumed.insert(prev_locs.clone());
                start_locs = prev_locs;
                backward_ext += 1;
            } else {
                break;
            }
        }

        // Extend forward from the original (not start) locations
        let mut current_locs = locations.clone();
        let mut forward_ext = 0usize;
        loop {
            let next_locs: Vec<(usize, usize)> =
                current_locs.iter().map(|(f, o)| (*f, o + 1)).collect();
            if location_to_hash.contains_key(&next_locs) {
                consumed.insert(next_locs.clone());
                current_locs = next_locs;
                forward_ext += 1;
            } else {
                break;
            }
        }

        let block_size = min_lines + backward_ext + forward_ext;

        // Build the DuplicateGroup from the extended start locations
        let mut dup_locations = Vec::new();
        let mut sample = Vec::new();

        for (file_idx, offset) in &start_locs {
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

        groups.push(DuplicateGroup {
            locations: dup_locations,
            line_count: block_size,
            sample,
            severity,
        });
    }

    // Sort by severity (Critical first), then by duplicated lines descending
    groups.sort_by(|a, b| match a.severity.cmp(&b.severity) {
        std::cmp::Ordering::Equal => b.duplicated_lines().cmp(&a.duplicated_lines()),
        other => other,
    });
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(path: &str, lines: &[(usize, &str)]) -> NormalizedFile {
        NormalizedFile {
            path: PathBuf::from(path),
            lines: lines
                .iter()
                .map(|(num, content)| NormalizedLine {
                    original_line_number: *num,
                    content: content.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn detect_exact_duplicate_two_files() {
        let files = vec![
            make_file(
                "a.rs",
                &[
                    (1, "fn foo() {"),
                    (2, "let x = 1;"),
                    (3, "let y = 2;"),
                    (4, "let z = x + y;"),
                    (5, "println!(\"{}\", z);"),
                    (6, "}"),
                ],
            ),
            make_file(
                "b.rs",
                &[
                    (1, "fn bar() {"),
                    (2, "let a = 10;"),
                    (3, "fn foo() {"),
                    (4, "let x = 1;"),
                    (5, "let y = 2;"),
                    (6, "let z = x + y;"),
                    (7, "println!(\"{}\", z);"),
                    (8, "}"),
                ],
            ),
        ];

        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].line_count, 6);
        assert_eq!(groups[0].locations.len(), 2);
    }

    #[test]
    fn no_duplicates_different_code() {
        let files = vec![
            make_file(
                "a.rs",
                &[
                    (1, "fn foo() {"),
                    (2, "let x = 1;"),
                    (3, "let y = 2;"),
                    (4, "let z = x + y;"),
                    (5, "println!(\"{}\", z);"),
                    (6, "}"),
                ],
            ),
            make_file(
                "b.rs",
                &[
                    (1, "fn bar() {"),
                    (2, "let a = 10;"),
                    (3, "let b = 20;"),
                    (4, "let c = a * b;"),
                    (5, "println!(\"{}\", c);"),
                    (6, "}"),
                ],
            ),
        ];

        let groups = detect_duplicates(&files, 6, false);
        assert!(groups.is_empty());
    }

    #[test]
    fn file_too_short_for_window() {
        let files = vec![make_file(
            "a.rs",
            &[(1, "fn foo() {"), (2, "let x = 1;"), (3, "}")],
        )];

        let groups = detect_duplicates(&files, 6, false);
        assert!(groups.is_empty());
    }

    #[test]
    fn detects_larger_block_via_extension() {
        // 8-line duplicate with window=6 should merge into one block of 8
        let code: Vec<(usize, &str)> = vec![
            (1, "fn process() {"),
            (2, "let a = read_input();"),
            (3, "let b = validate(a);"),
            (4, "let c = transform(b);"),
            (5, "let d = serialize(c);"),
            (6, "write_output(d);"),
            (7, "log(\"done\");"),
            (8, "}"),
        ];

        let files = vec![make_file("a.rs", &code), make_file("b.rs", &code)];

        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].line_count, 8);
    }

    #[test]
    fn backward_extension_finds_full_block() {
        // File b has extra lines before the duplicate. With backward extension,
        // the detector should still find the full 7-line block even if the
        // initial match window starts mid-block.
        let files = vec![
            make_file(
                "a.rs",
                &[
                    (1, "fn setup() {"),
                    (2, "let config = load();"),
                    (3, "let db = connect(config);"),
                    (4, "let cache = init_cache();"),
                    (5, "let server = build(db, cache);"),
                    (6, "server.start();"),
                    (7, "}"),
                ],
            ),
            make_file(
                "b.rs",
                &[
                    (1, "fn setup() {"),
                    (2, "let config = load();"),
                    (3, "let db = connect(config);"),
                    (4, "let cache = init_cache();"),
                    (5, "let server = build(db, cache);"),
                    (6, "server.start();"),
                    (7, "}"),
                ],
            ),
        ];

        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].line_count, 7);
    }

    #[test]
    fn three_way_duplicate() {
        let code: Vec<(usize, &str)> = vec![
            (1, "fn process() {"),
            (2, "let data = read();"),
            (3, "let result = transform(data);"),
            (4, "write(result);"),
            (5, "log(\"done\");"),
            (6, "}"),
        ];

        let files = vec![
            make_file("a.rs", &code),
            make_file("b.rs", &code),
            make_file("c.rs", &code),
        ];

        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].locations.len(), 3);
        assert_eq!(groups[0].duplicated_lines(), 12); // 6 * (3-1)
    }

    #[test]
    fn duplicate_within_same_file() {
        let files = vec![make_file(
            "a.rs",
            &[
                (1, "fn foo() {"),
                (2, "let x = 1;"),
                (3, "let y = 2;"),
                (4, "let z = x + y;"),
                (5, "println!(\"{}\", z);"),
                (6, "}"),
                (10, "fn foo() {"),
                (11, "let x = 1;"),
                (12, "let y = 2;"),
                (13, "let z = x + y;"),
                (14, "println!(\"{}\", z);"),
                (15, "}"),
            ],
        )];

        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].locations.len(), 2);
    }

    #[test]
    fn sample_contains_up_to_5_lines() {
        let code: Vec<(usize, &str)> = vec![
            (1, "fn a() {"),
            (2, "let x = 1;"),
            (3, "let y = 2;"),
            (4, "let z = 3;"),
            (5, "let w = 4;"),
            (6, "let v = 5;"),
            (7, "let u = 6;"),
            (8, "let t = 7;"),
            (9, "println!(\"{}\", x);"),
            (10, "}"),
        ];

        let files = vec![make_file("a.rs", &code), make_file("b.rs", &code)];

        let groups = detect_duplicates(&files, 6, false);
        assert!(!groups.is_empty());
        assert!(groups[0].sample.len() <= 5);
    }

    #[test]
    fn two_occurrences_is_tolerable() {
        let code: Vec<(usize, &str)> = vec![
            (1, "fn process() {"),
            (2, "let data = read();"),
            (3, "let result = transform(data);"),
            (4, "write(result);"),
            (5, "log(\"done\");"),
            (6, "}"),
        ];
        let files = vec![make_file("a.rs", &code), make_file("b.rs", &code)];
        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].severity, DuplicationSeverity::Tolerable);
    }

    #[test]
    fn three_occurrences_is_critical() {
        let code: Vec<(usize, &str)> = vec![
            (1, "fn process() {"),
            (2, "let data = read();"),
            (3, "let result = transform(data);"),
            (4, "write(result);"),
            (5, "log(\"done\");"),
            (6, "}"),
        ];
        let files = vec![
            make_file("a.rs", &code),
            make_file("b.rs", &code),
            make_file("c.rs", &code),
        ];
        let groups = detect_duplicates(&files, 6, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].severity, DuplicationSeverity::Critical);
    }

    #[test]
    fn critical_sorted_before_tolerable() {
        // 3 files: a+b+c share block1 (critical), a+b share block2 (tolerable)
        let files = vec![
            make_file(
                "a.rs",
                &[
                    (1, "fn shared3() {"),
                    (2, "let x = 1;"),
                    (3, "let y = 2;"),
                    (4, "let z = 3;"),
                    (5, "let w = 4;"),
                    (6, "}"),
                    (10, "fn shared2() {"),
                    (11, "let a = 10;"),
                    (12, "let b = 20;"),
                    (13, "let c = 30;"),
                    (14, "let d = 40;"),
                    (15, "let e = 50;"),
                    (16, "}"),
                ],
            ),
            make_file(
                "b.rs",
                &[
                    (1, "fn shared3() {"),
                    (2, "let x = 1;"),
                    (3, "let y = 2;"),
                    (4, "let z = 3;"),
                    (5, "let w = 4;"),
                    (6, "}"),
                    (10, "fn shared2() {"),
                    (11, "let a = 10;"),
                    (12, "let b = 20;"),
                    (13, "let c = 30;"),
                    (14, "let d = 40;"),
                    (15, "let e = 50;"),
                    (16, "}"),
                ],
            ),
            make_file(
                "c.rs",
                &[
                    (1, "fn shared3() {"),
                    (2, "let x = 1;"),
                    (3, "let y = 2;"),
                    (4, "let z = 3;"),
                    (5, "let w = 4;"),
                    (6, "}"),
                ],
            ),
        ];
        let groups = detect_duplicates(&files, 6, false);
        assert!(groups.len() >= 2);
        // First group should be Critical (3 occurrences)
        assert_eq!(groups[0].severity, DuplicationSeverity::Critical);
        // Find a Tolerable group
        let has_tolerable = groups
            .iter()
            .any(|g| g.severity == DuplicationSeverity::Tolerable);
        assert!(has_tolerable);
    }

    #[test]
    fn fnv_hash_is_deterministic() {
        let line = NormalizedLine {
            original_line_number: 1,
            content: "let x = 42;".to_string(),
        };
        let h1 = hash_window(&[line]);
        let line2 = NormalizedLine {
            original_line_number: 1,
            content: "let x = 42;".to_string(),
        };
        let h2 = hash_window(&[line2]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn fnv_hash_different_content() {
        let a = NormalizedLine {
            original_line_number: 1,
            content: "let x = 1;".to_string(),
        };
        let b = NormalizedLine {
            original_line_number: 1,
            content: "let x = 2;".to_string(),
        };
        assert_ne!(hash_window(&[a]), hash_window(&[b]));
    }
}
