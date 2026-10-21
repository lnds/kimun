//! Long help of `km docs refs`.

/// What the command checks and how `--since-ref` narrows it.
pub const REFS: &str = "\
Find references in Markdown documents that lead nowhere.

Checked:
  Links and images   [text](path), ![alt](path), [label]: path, <a href>,
                     <img src>: the file must exist, from the document or,
                     for a path starting with /, from the repository root
  Anchors            path#heading and #heading: the heading must exist in
                     the Markdown document linked, named as GitHub names it
  Paths in code      `src/kind.rs`, `scripts/`: a code span shaped like a
                     relative path. Alive when it exists from the document,
                     from the root, or as the end of a path of the
                     repository; dead when the history held it once. Outside
                     git, read when it starts at a name of the tree

URLs are not fetched; an absolute link without extension is a route of a
site, not a file. Fenced and indented code, front matter and HTML comments
are not read. Files ignored by git do not count as paths of the repository.

--since-ref REF keeps the documents changed since REF and, in the others,
the references to files changed since it: a pull request that deletes a
file learns which documents still name it.

Examples:
  km docs refs
  km docs refs --since-ref origin/main --fail-on-dead
  km docs refs --format github";
