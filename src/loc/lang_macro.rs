/// The `lang!` macro for declaring language specifications.
///
/// Provides a compact syntax for defining `LanguageSpec` instances with
/// various combinations of line comments, block comments, string handling,
/// pragmas, and shebang detection.
///
/// # Variants
///
/// - `line: + block: + pragma:` — full specification (e.g., Haskell)
/// - `line: + block:` — line and block comments (e.g., Rust, C)
/// - `line:` only — line comment languages (e.g., Python, Shell)
/// - `block: + pragma:` — block comments with pragmas, no line comments
/// - `block:` only — block comment languages (e.g., HTML, CSS)
/// - `lines: [...]` — multiple line comment markers (e.g., DOS Batch)
/// - `none` — no comment syntax (e.g., JSON, Markdown)
///
/// # Optional flags
///
/// - `nested: true` — enable nested block comment tracking
/// - `sq: true` — treat single quotes as string delimiters
/// - `tq: true` — enable triple-quote string support
/// - `shebangs: [...]` — shebang interpreter names
macro_rules! lang_spec {
    ($name:expr, ext: [$($ext:expr),*], files: [$($f:expr),*], $($rest:tt)*) => {
        lang_spec!(@build $name, &[$($ext),*], &[$($f),*], $($rest)*)
    };
    ($name:expr, ext: [$($ext:expr),*], $($rest:tt)*) => {
        lang_spec!(@build $name, &[$($ext),*], &[], $($rest)*)
    };
    // line + block + pragma
    (@build $name:expr, $ext:expr, $files:expr,
     line: $lc:expr, block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     , pragma: $po:expr, $pc:expr
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$lc],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: Some(($po, $pc)),
            shebangs: &[$($($sh),*)?],
        }
    };
    // line + block (no pragma)
    (@build $name:expr, $ext:expr, $files:expr,
     line: $lc:expr, block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$lc],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // line comment only
    (@build $name:expr, $ext:expr, $files:expr,
     line: $lc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$lc],
            line_comment_not_before: "",
            block_comment: None,
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // block + pragma (no line comment)
    (@build $name:expr, $ext:expr, $files:expr,
     block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     , pragma: $po:expr, $pc:expr
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: Some(($po, $pc)),
            shebangs: &[$($($sh),*)?],
        }
    };
    // block only (no pragma, no line comment)
    (@build $name:expr, $ext:expr, $files:expr,
     block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // multiple line comment markers (e.g. DOS Batch: :: and rem)
    (@build $name:expr, $ext:expr, $files:expr,
     lines: [$($lc:expr),+]
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$($lc),+],
            line_comment_not_before: "",
            block_comment: None,
            nested_block_comments: false,
            single_quote_strings: false,
            triple_quote_strings: false,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // no comments
    (@build $name:expr, $ext:expr, $files:expr,
     none
     $(, sq: $sq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[],
            line_comment_not_before: "",
            block_comment: None,
            nested_block_comments: false,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
}

pub(super) use lang_spec;
