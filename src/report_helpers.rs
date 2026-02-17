use std::path::Path;

use serde::Serialize;

/// Compute the max display width for paths, with a minimum of `min`.
pub fn max_path_width<'a>(paths: impl Iterator<Item = &'a Path>, min: usize) -> usize {
    paths
        .map(|p| p.display().to_string().len())
        .max()
        .unwrap_or(min)
        .max(min)
}

/// Print a horizontal separator of box-drawing chars.
pub fn separator(width: usize) -> String {
    "\u{2500}".repeat(width)
}

/// Serialize to pretty JSON and print to stdout.
pub fn print_json_stdout(value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn max_path_width_with_paths() {
        let paths = vec![
            PathBuf::from("src/foo.rs"),
            PathBuf::from("src/very_long_name.rs"),
        ];
        let w = max_path_width(paths.iter().map(|p| p.as_path()), 4);
        assert_eq!(w, "src/very_long_name.rs".len());
    }

    #[test]
    fn max_path_width_empty() {
        let paths: Vec<PathBuf> = vec![];
        let w = max_path_width(paths.iter().map(|p| p.as_path()), 4);
        assert_eq!(w, 4);
    }

    #[test]
    fn max_path_width_min_enforced() {
        let paths = vec![PathBuf::from("a")];
        let w = max_path_width(paths.iter().map(|p| p.as_path()), 10);
        assert_eq!(w, 10);
    }

    #[test]
    fn separator_width() {
        let s = separator(5);
        // Each ─ is 3 bytes in UTF-8
        assert_eq!(s.chars().count(), 5);
    }

    #[test]
    fn print_json_stdout_works() {
        let data = vec![1, 2, 3];
        print_json_stdout(&data).unwrap();
    }
}
