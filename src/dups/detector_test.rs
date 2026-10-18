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

#[test]
fn boilerplate_over_max_occurrences_is_skipped() {
    // Create MAX_OCCURRENCES + 1 files all containing the same 6-line block.
    // The validate_hashes phase should skip this pattern (> MAX_OCCURRENCES).
    let code: &[(usize, &str)] = &[
        (1, "fn boilerplate() {"),
        (2, "let x = 1;"),
        (3, "let y = 2;"),
        (4, "let z = x + y;"),
        (5, "println!(\"{}\", z);"),
        (6, "}"),
    ];
    // 101 files with identical content
    let files: Vec<NormalizedFile> = (0..=MAX_OCCURRENCES)
        .map(|i| make_file(&format!("file_{i}.rs"), code))
        .collect();

    // quiet=false so the skipped_common eprintln branch is exercised
    let groups = detect_duplicates(&files, 6, false);
    // The boilerplate should be skipped, so no groups
    assert!(
        groups.is_empty(),
        "boilerplate with >{MAX_OCCURRENCES} occurrences should be skipped, got {} groups",
        groups.len()
    );
}

#[test]
fn boilerplate_quiet_mode_no_output() {
    // Same as above but with quiet=true — exercises the !quiet branch (negated)
    let code: &[(usize, &str)] = &[
        (1, "fn common() {"),
        (2, "return 0;"),
        (3, "let a = 1;"),
        (4, "let b = 2;"),
        (5, "let c = 3;"),
        (6, "}"),
    ];
    let files: Vec<NormalizedFile> = (0..=MAX_OCCURRENCES)
        .map(|i| make_file(&format!("q_{i}.rs"), code))
        .collect();

    // quiet=true — the eprintln inside validate_hashes is suppressed
    let groups = detect_duplicates(&files, 6, true);
    assert!(
        groups.is_empty(),
        "boilerplate should be skipped in quiet mode too"
    );
}

/// A file of `lines`, numbered from 1.
fn numbered(path: &str, lines: &[&str]) -> NormalizedFile {
    let pairs: Vec<(usize, &str)> = lines.iter().enumerate().map(|(i, l)| (i + 1, *l)).collect();
    make_file(path, &pairs)
}

#[test]
fn a_run_of_repeated_lines_is_no_duplicate_of_itself() {
    let mut lines = vec!["0xFF, 0xFF, 0xFF, 0xFF,"; 8];
    lines.insert(0, "static const unsigned char T[] = {");
    lines.push("62, 63,");
    lines.push("};");
    let groups = detect_duplicates(&[numbered("table.c", &lines)], 6, true);
    assert!(groups.is_empty(), "{groups:?}");
}

#[test]
fn occurrences_of_a_group_never_overlap() {
    let x = ["x1();", "x2();", "x3();"];
    // x x x in one file: a 6-line block (x x) can only be found once at
    // offsets 0 and 3, never at 0 and 1.
    let lines: Vec<&str> = x.iter().chain(&x).chain(&x).copied().collect();
    let groups = detect_duplicates(&[numbered("a.c", &lines)], 3, true);
    for g in &groups {
        let mut starts = g.starts.clone();
        starts.sort();
        for pair in starts.windows(2) {
            assert!(pair[1].1 >= pair[0].1 + g.line_count, "{g:?}");
        }
    }
}

#[test]
fn a_block_extends_past_a_window_found_elsewhere_too() {
    let a: Vec<String> = (1..=6).map(|i| format!("int a{i} = f({i});")).collect();
    let b: Vec<String> = (1..=6).map(|i| format!("int b{i} = g({i});")).collect();
    let both: Vec<&str> = a.iter().chain(&b).map(String::as_str).collect();
    let only_b: Vec<&str> = b.iter().map(String::as_str).collect();
    let files = [
        numbered("one.c", &both),
        numbered("two.c", &both),
        numbered("three.c", &only_b),
    ];
    let groups = detect_duplicates(&files, 6, true);
    let mut sizes: Vec<(usize, usize)> = groups
        .iter()
        .map(|g| (g.line_count, g.locations.len()))
        .collect();
    sizes.sort();
    assert_eq!(sizes, [(6, 3), (12, 2)]);
    // two.c whole (12) and one more copy of B (6), each line once.
    assert_eq!(duplicated_line_count(&groups), 18);
}

#[test]
fn duplicated_lines_never_exceed_the_code() {
    let lines = vec!["same();"; 20];
    let files = [numbered("a.c", &lines), numbered("b.c", &lines)];
    let groups = detect_duplicates(&files, 6, true);
    assert!(duplicated_line_count(&groups) <= 40);
}
