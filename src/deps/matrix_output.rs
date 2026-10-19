//! `km deps --matrix`: the matrix printed on a terminal, as JSON, or
//! written as an HTML page.

use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use super::{analyze, matrix, matrix_html, matrix_json, matrix_text, report, show_paths};
use crate::cli::OutputMode;
use crate::walk::WalkConfig;

/// Where the matrix goes.
enum Sink<'a> {
    Text,
    Json,
    Html(&'a Path),
}

fn sink<'a>(
    output: OutputMode,
    args: &'a crate::cli::DepsArgs,
) -> Result<Sink<'a>, Box<dyn Error>> {
    match (output, &args.html) {
        (OutputMode::Table, Some(file)) => Ok(Sink::Html(file)),
        (OutputMode::Table, None) => Ok(Sink::Text),
        (OutputMode::Json, None) => Ok(Sink::Json),
        _ => Err("--matrix prints a table or JSON, or with --html a page".into()),
    }
}

/// Print the dependency structure matrix of the analyzed files, or of
/// those in cycles.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    args: &crate::cli::DepsArgs,
    expand: &matrix::Expand,
) -> Result<(), Box<dyn Error>> {
    let sink = sink(output, args)?;
    let mut result = analyze(cfg);
    show_paths(&mut result, cfg.path);
    let in_matrix = result
        .entries
        .iter()
        .filter(|e| !args.cycles_only || e.in_cycle);
    let files: Vec<PathBuf> = in_matrix.map(|e| e.path.clone()).collect();
    let m = (!files.is_empty())
        .then(|| matrix::Matrix::build(&files, &result.edges, &result.cycles, expand))
        .transpose()?;
    if let Sink::Json = sink {
        let out = matrix_json::of(m.as_ref(), &result.unsupported);
        return crate::report_helpers::print_json_stdout(&out);
    }
    match (&m, sink) {
        (None, _) => println!("No source files found for dependency analysis."),
        (Some(m), Sink::Html(file)) => write_matrix_html(m, cfg.path, file)?,
        (Some(m), _) => print_matrix(m)?,
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
