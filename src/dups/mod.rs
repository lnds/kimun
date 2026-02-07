mod detector;
mod report;

use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use ignore::WalkBuilder;

use crate::loc::counter::{classify_reader, LineKind};
use crate::loc::language::{detect, detect_by_shebang, LanguageSpec};
use detector::{detect_duplicates, NormalizedFile, NormalizedLine};
use report::{print_detailed, print_summary, DuplicationMetrics};

fn normalize_file(path: &Path, spec: &LanguageSpec) -> Result<Option<NormalizedFile>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Binary detection on first 512 bytes (without reading entire file)
    let mut header = [0u8; 512];
    let n = reader.read(&mut header)?;
    if header[..n].contains(&0) {
        return Ok(None);
    }
    reader.seek(SeekFrom::Start(0))?;

    // Read all lines once
    let lines: Vec<String> = reader
        .lines()
        .map_while(Result::ok)
        .collect();

    // Classify using a Cursor over the joined content
    let content = lines.join("\n");
    let kinds = classify_reader(BufReader::new(Cursor::new(content)), spec);

    let normalized: Vec<NormalizedLine> = lines
        .iter()
        .zip(kinds.iter())
        .enumerate()
        .filter(|(_, (_, kind))| **kind == LineKind::Code)
        .map(|(i, (line, _))| NormalizedLine {
            original_line_number: i + 1,
            content: line.trim().to_string(),
        })
        .collect();

    Ok(Some(NormalizedFile {
        path: path.to_path_buf(),
        lines: normalized,
    }))
}

fn try_detect_shebang(path: &Path) -> Option<&'static LanguageSpec> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;
    detect_by_shebang(&first_line)
}

pub fn run(
    path: &Path,
    min_lines: usize,
    show_report: bool,
    show_all: bool,
) -> Result<(), Box<dyn Error>> {
    let mut files: Vec<NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    let walker = WalkBuilder::new(path)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|entry| {
            !(entry.file_type().is_some_and(|ft| ft.is_dir())
                && entry.file_name() == ".git")
        })
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("warning: {err}");
                continue;
            }
        };

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let file_path = entry.path();
        let spec = match detect(file_path) {
            Some(s) => s,
            None => match try_detect_shebang(file_path) {
                Some(s) => s,
                None => continue,
            },
        };

        match normalize_file(file_path, spec) {
            Ok(Some(nf)) => {
                total_code_lines += nf.lines.len();
                files.push(nf);
            }
            Ok(None) => {} // binary, skip
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    if files.is_empty() {
        println!("No recognized source files found.");
        return Ok(());
    }

    let groups = detect_duplicates(&files, min_lines);

    let duplicated_lines: usize = groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = groups.iter().map(|g| g.line_count).max().unwrap_or(0);

    let files_with_dups: HashSet<&Path> = groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();

    let metrics = DuplicationMetrics {
        total_code_lines,
        duplicated_lines,
        duplicate_groups: groups.len(),
        files_with_duplicates: files_with_dups.len(),
        largest_block,
    };

    if show_report {
        print_detailed(&metrics, &groups, show_all);
    } else {
        print_summary(&metrics);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn run_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), 6, false, false).unwrap();
    }

    #[test]
    fn run_with_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "fn foo() {\n    let x = 1;\n    let y = 2;\n    let z = x + y;\n    println!(\"{}\", z);\n    return z;\n}\n",
        ).unwrap();
        fs::write(
            dir.path().join("b.rs"),
            "fn bar() {\n    let a = 10;\n    let b = 20;\n    let c = a * b;\n    println!(\"{}\", c);\n    return c;\n}\n",
        ).unwrap();
        run(dir.path(), 6, false, false).unwrap();
    }

    #[test]
    fn run_detects_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        // Should not panic, should detect duplicates
        run(dir.path(), 6, false, false).unwrap();
    }

    #[test]
    fn run_with_report_flag() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        run(dir.path(), 6, true, false).unwrap();
    }

    #[test]
    fn run_with_show_all_flag() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        run(dir.path(), 6, true, true).unwrap();
    }

    #[test]
    fn run_skips_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), 6, false, false).unwrap();
    }

    #[test]
    fn run_with_high_min_lines() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn f() {\n    let x = 1;\n    let y = 2;\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        // min_lines=20 means no 4-line file can produce duplicates
        run(dir.path(), 20, false, false).unwrap();
    }

    #[test]
    fn normalize_file_skips_comments_and_blanks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(
            &path,
            "// comment\n\nfn main() {\n    // another comment\n    let x = 1;\n}\n",
        ).unwrap();

        let spec = detect(Path::new("test.rs")).unwrap();
        let nf = normalize_file(&path, spec).unwrap().unwrap();

        // Should only have code lines: "fn main() {", "let x = 1;", "}"
        assert_eq!(nf.lines.len(), 3);
        assert_eq!(nf.lines[0].content, "fn main() {");
        assert_eq!(nf.lines[1].content, "let x = 1;");
        assert_eq!(nf.lines[2].content, "}");
    }

    #[test]
    fn normalize_file_binary_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.c");
        fs::write(&path, b"hello\x00world").unwrap();

        let spec = detect(Path::new("test.c")).unwrap();
        assert!(normalize_file(&path, spec).unwrap().is_none());
    }
}
