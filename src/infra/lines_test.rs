use super::*;

fn parts(text: &str, format: Format) -> Vec<(&str, &str)> {
    split(text, format)
        .into_iter()
        .map(|l| (l.code.trim(), l.comment.trim()))
        .collect()
}

#[test]
fn hcl_comments_heredocs_and_blocks() {
    let text = "a = \"x#y\" # note\n\
                b = 1 // other\n\
                /* start\n\
                still */ c = 2\n\
                policy = <<EOF\n\
                # not a comment\n\
                EOF\n\
                d = 3 # after\n";
    assert_eq!(
        parts(text, Format::Hcl),
        [
            ("a = \"x#y\"", "# note"),
            ("b = 1", "// other"),
            ("", "/* start"),
            ("c = 2", "still */"),
            ("policy = <<EOF", ""),
            ("# not a comment", ""),
            ("EOF", ""),
            ("d = 3", "# after"),
        ]
    );
}

#[test]
fn yaml_comments_outside_quotes() {
    let text = "a: \"x # y\" # real\nb: it's # c\nurl: http://x/#frag\n# whole\n";
    assert_eq!(
        parts(text, Format::Yaml),
        [
            ("a: \"x # y\"", "# real"),
            ("b: it's", "# c"),
            ("url: http://x/#frag", ""),
            ("", "# whole"),
        ]
    );
}

#[test]
fn dockerfile_comments_are_whole_lines() {
    let text = "# syntax\nRUN echo ${A#b}\n";
    assert_eq!(
        parts(text, Format::Dockerfile),
        [("", "# syntax"), ("RUN echo ${A#b}", "")]
    );
}

#[test]
fn assignments_of_each_form() {
    let pairs = |code| -> Vec<(&str, &str, bool)> {
        assignments(code)
            .into_iter()
            .map(|(k, v)| (k, v.text, v.quoted))
            .collect()
    };
    assert_eq!(pairs("password: hunter2"), [("password", "hunter2", false)]);
    assert_eq!(pairs("  - DB_PASS=x"), [("DB_PASS", "x", false)]);
    assert_eq!(pairs("token = \"abc\","), [("token", "abc", true)]);
    assert_eq!(pairs("\"key\": 'v'"), [("key", "v", true)]);
    assert_eq!(
        pairs("ENV A=1 B=\"2\""),
        [("A", "1", false), ("B", "2", true)]
    );
    assert_eq!(
        pairs("ENV SECRET value here"),
        [("SECRET", "value here", false)]
    );
    assert_eq!(pairs("url: http://x"), [("url", "http://x", false)]);
    assert!(pairs("if a == b").is_empty());
    assert!(pairs("RUN make").is_empty());
}
