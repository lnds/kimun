//! `km deps --matrix`: the matrix printed on a terminal or written as an
//! HTML page.

use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use super::{analyze, matrix, matrix_html, matrix_text, report, show_paths};
use crate::walk::WalkConfig;

/// Print the dependency structure matrix of the analyzed files, or of
/// those in cycles.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: crate::cli::OutputMode,
    args: &crate::cli::DepsArgs,
    expand: &matrix::Expand,
) -> Result<(), Box<dyn Error>> {
    if !matches!(output, crate::cli::OutputMode::Table) {
        return Err("--matrix prints a table; --format json lists the edges in `edges`".into());
    }
    let mut result = analyze(cfg);
    show_paths(&mut result, cfg.path);
    let files: Vec<PathBuf> = result
        .entries
        .iter()
        .filter(|e| !args.cycles_only || e.in_cycle)
        .map(|e| e.path.clone())
        .collect();
    if files.is_empty() {
        println!("No source files found for dependency analysis.");
    } else {
        let m = matrix::Matrix::build(&files, &result.edges, &result.cycles, expand)?;
        match &args.html {
            Some(file) => write_matrix_html(&m, cfg.path, file)?,
            None => print_matrix(&m)?,
        }
    }
    if let Some(note) = report::unsupported_note(&result.unsupported) {
        println!("{note}");
    }
    Ok(())
}

/// Write the matrix page of the directory `root` to `file`.
fn write_matrix_html(m: &matrix::Matrix, root: &Path, file: &Path) -> Result<(), Box<dyn Error>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let name = root
        .file_name()
        .map_or(String::from("."), |n| n.to_string_lossy().into_owned());
    std::fs::write(file, matrix_html::render(m, &name)?)
        .map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    println!("Wrote {}", file.display());
    Ok(())
}

/// Print a matrix, coloured on a terminal, unless it is wider than one.
fn print_matrix(m: &matrix::Matrix) -> Result<(), Box<dyn Error>> {
    let terminal = std::io::stdout().is_terminal();
    if let Some((terminal_size::Width(columns), _)) = terminal_size::terminal_size() {
        let wide = matrix_text::width(m);
        if terminal && wide > usize::from(columns) {
            return Err(format!(
                "the matrix takes {wide} columns and the terminal {columns}: try a lower --depth or --focus <DIR>"
            )
            .into());
        }
    }
    let color = terminal && std::env::var_os("NO_COLOR").is_none();
    print!("{}", matrix_text::render(m, color));
    Ok(())
}
