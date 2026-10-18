use super::*;
use crate::deps::javascript_config::{Config, Modules, Package};

#[test]
fn every_import_form_is_extracted() {
    let src = r#"
import foo from './foo';
import { bar } from '../bar';
import external from 'lodash';
import './side-effect';
import type { T } from '@/types';
export * from './barrel';
const x = require('./utils');
const lazy = await import('./lazy');
import legacy = require('./legacy');
"#;
    assert_eq!(
        extract(src),
        vec![
            "./foo",
            "../bar",
            "lodash",
            "./side-effect",
            "@/types",
            "./barrel",
            "./utils",
            "./lazy",
            "./legacy",
        ]
    );
}

#[test]
fn other_strings_are_not_imports() {
    let src =
        "const data = fs.readFileSync('./data.ts');\nArray.from('./x');\nobj.require('./y');\n";
    assert!(extract(src).is_empty());
}

#[test]
fn comments_are_skipped() {
    let src = "// import a from './line';\n/* import b from './block'; */\n/**\n * import c from './doc';\n */\n";
    assert!(extract(src).is_empty());
}

#[test]
fn several_imports_on_one_line() {
    let src = "export { a } from 'pkg'; export { b } from './local';\n";
    assert_eq!(extract(src), vec!["pkg", "./local"]);
}

#[test]
fn multi_line_import() {
    let src = "import {\n  a,\n  b,\n} from './many'\n";
    assert_eq!(extract(src), vec!["./many"]);
}

#[test]
fn interpolated_template_is_no_specifier() {
    let src = "const m = import(`./locale/${lang}`);\nconst n = import(`./fixed`);\n";
    assert_eq!(extract(src), vec!["./fixed"]);
}

#[test]
fn apostrophe_in_jsx_text_does_not_hide_the_next_import() {
    let src = "const a = <p>Don't</p>;\nimport b from './b';\n";
    assert_eq!(extract(src), vec!["./b"]);
}

fn layout(files: &[&str], configs: Vec<Config>, packages: Vec<Package>) -> Layout {
    Layout {
        known: files.iter().map(PathBuf::from).collect(),
        javascript: Modules { configs, packages },
        ..Layout::default()
    }
}

fn config(dir: &str, base_url: Option<&str>, paths: &[(&str, &[&str])]) -> Config {
    Config {
        dir: PathBuf::from(dir),
        base_url: base_url.map(PathBuf::from),
        paths: paths
            .iter()
            .map(|(p, ts)| (p.to_string(), ts.iter().map(|t| t.to_string()).collect()))
            .collect(),
    }
}

fn resolved(importer: &str, specifier: &str, layout: &Layout) -> Option<String> {
    resolve(Path::new(importer), specifier, layout).map(|p| p.display().to_string())
}

#[test]
fn relative_specifiers_resolve_from_the_importer() {
    let l = layout(
        &["src/a.ts", "src/lib/index.ts", "src/foo.service.ts"],
        vec![],
        vec![],
    );
    assert_eq!(resolved("src/b.ts", "./a", &l).as_deref(), Some("src/a.ts"));
    assert_eq!(
        resolved("src/b.ts", "./lib", &l).as_deref(),
        Some("src/lib/index.ts")
    );
    assert_eq!(
        resolved("src/b.ts", "./foo.service", &l).as_deref(),
        Some("src/foo.service.ts")
    );
}

#[test]
fn a_compiled_extension_names_the_typescript_source() {
    let l = layout(&["src/a.ts", "src/c.mts"], vec![], vec![]);
    assert_eq!(
        resolved("src/b.ts", "./a.js", &l).as_deref(),
        Some("src/a.ts")
    );
    assert_eq!(
        resolved("src/b.ts", "./c.mjs", &l).as_deref(),
        Some("src/c.mts")
    );
}

#[test]
fn paths_aliases_resolve() {
    let configs = vec![config(
        "web",
        None,
        &[
            ("@/*", &["web/src/*"]),
            ("~config", &["web/config/index.ts"]),
        ],
    )];
    let l = layout(
        &["web/src/components/Button.tsx", "web/config/index.ts"],
        configs,
        vec![],
    );
    assert_eq!(
        resolved("web/src/app.tsx", "@/components/Button", &l).as_deref(),
        Some("web/src/components/Button.tsx")
    );
    assert_eq!(
        resolved("web/src/app.tsx", "~config", &l).as_deref(),
        Some("web/config/index.ts")
    );
}

#[test]
fn the_longest_prefix_wins_and_targets_are_tried_in_order() {
    let configs = vec![config(
        "",
        None,
        &[
            ("@app/*", &["src/*"]),
            ("@app/ui/*", &["missing/*", "ui/*"]),
        ],
    )];
    let l = layout(&["src/ui/x.ts", "ui/x.ts"], configs, vec![]);
    assert_eq!(
        resolved("src/a.ts", "@app/ui/x", &l).as_deref(),
        Some("ui/x.ts")
    );
}

#[test]
fn aliases_apply_only_under_their_config() {
    let configs = vec![config("web", None, &[("@/*", &["web/src/*"])])];
    let l = layout(&["web/src/a.ts"], configs, vec![]);
    assert_eq!(resolved("api/b.ts", "@/a", &l), None);
}

#[test]
fn base_url_resolves_bare_specifiers() {
    let configs = vec![config("", Some("src"), &[])];
    let l = layout(&["src/components/Nav.jsx"], configs, vec![]);
    assert_eq!(
        resolved("src/app.jsx", "components/Nav", &l).as_deref(),
        Some("src/components/Nav.jsx")
    );
    assert_eq!(resolved("src/app.jsx", "react", &l), None);
}

fn package(dir: &str, name: &str, entries: &[&str]) -> Package {
    Package {
        dir: PathBuf::from(dir),
        name: name.to_string(),
        entries: entries.iter().map(|e| e.to_string()).collect(),
    }
}

#[test]
fn workspace_packages_resolve_by_name() {
    let packages = vec![
        package("packages/ui", "@acme/ui", &["./dist/index.js"]),
        package("packages/utils", "utils", &[]),
    ];
    let l = layout(
        &[
            "packages/ui/src/index.ts",
            "packages/ui/src/button.ts",
            "packages/utils/index.js",
        ],
        vec![],
        packages,
    );
    assert_eq!(
        resolved("apps/web/a.ts", "@acme/ui", &l).as_deref(),
        Some("packages/ui/src/index.ts")
    );
    assert_eq!(
        resolved("apps/web/a.ts", "@acme/ui/button", &l).as_deref(),
        Some("packages/ui/src/button.ts")
    );
    assert_eq!(
        resolved("apps/web/a.ts", "utils", &l).as_deref(),
        Some("packages/utils/index.js")
    );
    assert_eq!(resolved("apps/web/a.ts", "lodash", &l), None);
}

#[test]
fn a_specifier_cannot_leave_the_repository() {
    let l = layout(&["a.ts"], vec![], vec![]);
    assert_eq!(resolved("a.ts", "../../a", &l), None);
}

#[test]
fn a_query_names_the_same_file() {
    let l = layout(&["src/template/main.js"], vec![], vec![]);
    assert_eq!(
        resolved("src/download.ts", "./template/main.js?raw", &l).as_deref(),
        Some("src/template/main.js")
    );
}
