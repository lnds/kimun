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

/// What makes a document stale and how it is measured.
pub const STALE: &str = "\
Find documents behind the code they describe: the files a document covers or
names changed after the document last did.

A document describes a file when:
  it covers it    covers: in its front matter, or [docs] covers in
                  .kimun.toml
  it names it     a link, a path in a code span (whole or by its end), or
                  a directory of two names or more above it

Documents themselves are never described. Changelogs (CHANGELOG, CHANGES,
HISTORY) and the globs of [docs] ignore are left out.

Each document shows when it last changed, how far behind the last change of
what it describes it is, how many commits changed that since, and the files.
Furthest behind first. Committed history only: uncommitted work does not
count. Requires a git repository.

For a change not yet merged, km impact lists the documents it leaves behind.

Examples:
  km docs stale
  km docs stale kb/ --top 50
  km docs stale --format json";

/// What the graph of notes is made of and what is reported.
pub const LINKS: &str = "\
The links between Markdown documents as a graph of notes, as a knowledge
base, a docs site or a second brain is.

Links:
  [text](path.md)   a relative Markdown link, from the document, or from
                    the walked directory for a path starting with /
  [[Note]]          a wikilink (Obsidian, Foam, Logseq): by path from the
                    root, else by note name, without case or extension;
                    of several notes of that name, the nearest
  URLs and anchors alone link to no note; links to files that are no
  document are not edges.

Reported:
  Broken links   no file has the path, no note the name
  Orphans        notes no note links to, besides entry points (README,
                 index, SUMMARY, CLAUDE, AGENTS, ...)
  Hubs           the notes most linked to
  Islands        groups of notes linked among themselves and to no other

km deps --kind docs draws the same graph with the dependency tools:
--matrix, cycles, fan-in and fan-out.

Examples:
  km docs links
  km docs links kb/ --top 50
  km docs links --format json";

/// Which files steer agents and what is measured of them.
pub const CONTEXT: &str = "\
Measure the files that steer coding agents, loaded into the model's context
on every task: models use long contexts worse, and worst what sits in the
middle (Liu et al., Lost in the Middle, 2024).

Files:
  instructions   CLAUDE.md, CLAUDE.local.md, AGENTS.md, GEMINI.md, read for
                 every task under their directory
  rules          .cursorrules, .cursor/rules/, .windsurfrules,
                 .github/copilot-instructions.md, .github/instructions/
  skill          .claude/skills/, SKILL.md
  agent          .claude/agents/
  command        .claude/commands/

Measured:
  Tokens     approximate: characters / 4
  Loads      what an agent working in a directory reads together: the
             instructions from the analyzed directory down to it
  Rules      lines of prose that ask (must, never, always, do not, should,
             debe, nunca, siempre, ...): how much is being asked
  Repeated   lines of prose found in more than one context file
  Dead       references km docs refs finds dead
  Commits    commits and last change of each file, in a git repository

No model is called. Files ignored by git are not read: .claude/ often is.

--max-tokens N fails when a load goes over N tokens.

Examples:
  km docs context
  km docs context --max-tokens 8000
  km docs context --format json";
