use super::*;

#[test]
fn block_import() {
    let src = r#"
import (
    "fmt"
    "github.com/user/project/pkg/foo"
    alias "github.com/user/project/internal/bar"
)
"#;
    assert_eq!(
        read(src).imports,
        vec![
            "fmt",
            "github.com/user/project/pkg/foo",
            "github.com/user/project/internal/bar",
        ]
    );
}

#[test]
fn single_line_imports() {
    let src = "package main\n\nimport \"fmt\"\nimport bar \"github.com/user/project/bar\"\n";
    assert_eq!(
        read(src).imports,
        vec!["fmt", "github.com/user/project/bar"]
    );
}

#[test]
fn strings_outside_imports_are_not_imports() {
    let src = "package main\nconst name = \"before\"\nimport (\n\t\"fmt\"\n\n\t\"os\"\n)\nvar after = \"after\"\n";
    assert_eq!(read(src).imports, vec!["fmt", "os"]);
}

#[test]
fn imports_in_comments_and_literals_are_not_imports() {
    let src = r#"/*
Package app does things.

	import "example.com/app"
*/
package app

// import "example.com/app/commented"

import (
	"fmt" // import "example.com/app/trailing"
	/* "example.com/app/block" */
	_ "example.com/app/driver"
)

import . `example.com/app/raw`

var usage = `
import "example.com/app/in_raw_string"
`
var quote = '"'
var s = "import \"example.com/app/in_string\""
"#;
    let file = read(src);
    assert_eq!(file.package.as_deref(), Some("app"));
    assert_eq!(
        file.imports,
        ["fmt", "example.com/app/driver", "example.com/app/raw"]
    );
}

#[test]
fn the_canonical_import_comment_is_no_import() {
    let file = read("package app // import \"example.com/app\"\n");
    assert!(file.imports.is_empty());
    assert!(!file.is_external_test());
    assert!(read("package app_test\n").is_external_test());
}
