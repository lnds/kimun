use super::*;

/// The words of each line, constants and keywords alike.
fn words(source: &str) -> Vec<Vec<String>> {
    lines(source)
        .into_iter()
        .map(|line| {
            line.tokens
                .into_iter()
                .filter_map(|t| match t {
                    Tok::Word(w) => Some(w),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

fn strings(source: &str) -> Vec<(String, bool)> {
    lines(source)
        .into_iter()
        .flat_map(|line| line.tokens)
        .filter_map(|t| match t {
            Tok::Str(text, interpolated) => Some((text, interpolated)),
            _ => None,
        })
        .collect()
}

fn all_words(source: &str) -> Vec<String> {
    words(source).into_iter().flatten().collect()
}

#[test]
fn comments_give_no_tokens() {
    let src = "# Foo::Bar\nx = 1 # Baz\n=begin\nQux\n=end\nY\n";
    assert_eq!(
        words(src),
        vec![vec![], vec!["x"], vec![], vec![], vec![], vec!["Y"], vec![]]
    );
}

#[test]
fn strings_are_one_token_with_their_text() {
    let src = "require 'a/b'\nputs \"x #{Foo} y\"\n";
    assert_eq!(all_words(src), vec!["require", "puts", "Foo"]);
    assert_eq!(
        strings(src),
        vec![("a/b".to_string(), false), ("x  y".to_string(), true)]
    );
}

#[test]
fn heredoc_bodies_are_text_and_keep_lines() {
    let src = "x = <<~SQL\n  SELECT Foo\n  SQL\nBar\ny = <<-'EOS'\nBaz\n  EOS\nQux\n";
    let words = words(src);
    assert_eq!(words[0], vec!["x"]);
    assert!(words[1].is_empty() && words[2].is_empty());
    assert_eq!(words[3], vec!["Bar"]);
    assert_eq!(words[7], vec!["Qux"]);
    assert_eq!(strings(src)[0].0, "  SELECT Foo\n");
}

#[test]
fn two_heredocs_on_one_line() {
    let src = "call(<<~A, <<~B)\n  Foo\nA\n  Bar\nB\nBaz\n";
    let words = words(src);
    assert_eq!(words[0], vec!["call"]);
    assert_eq!(words[5], vec!["Baz"]);
}

#[test]
fn shift_is_not_a_heredoc() {
    assert_eq!(
        all_words("a << B\nitems <<x\n"),
        vec!["a", "B", "items", "x"]
    );
}

#[test]
fn percent_literals_and_regexes() {
    let src = "a = %w[Foo Bar]\nb = %q(x (Y) z)\nc = x =~ /Foo[/]Bar/i\nd = 4 / Two / 2\ne = %{A}\nf = 10 % Mod\n";
    assert_eq!(
        all_words(src),
        vec!["a", "b", "c", "x", "d", "Two", "e", "f", "Mod"]
    );
}

#[test]
fn symbols_variables_and_characters() {
    let src = "autoload :Foo, 'foo'\n@Bar = $Baz\nc = ?\"\nD::E\n";
    let tokens: Vec<Tok> = lines(src).into_iter().flat_map(|l| l.tokens).collect();
    assert!(tokens.contains(&Tok::Symbol("Foo".to_string())));
    assert!(!tokens.contains(&Tok::Word("Bar".to_string())));
    assert!(tokens.contains(&Tok::Scope));
    assert_eq!(all_words(src), vec!["autoload", "c", "D", "E"]);
}

#[test]
fn end_marker_stops_the_code() {
    assert_eq!(all_words("A\n__END__\nB\n"), vec!["A"]);
}

#[test]
fn indent_is_the_column_of_the_first_token() {
    let lines = lines("class A\n  def b\n  end\nend\n");
    let indents: Vec<usize> = lines.iter().map(|l| l.indent).collect();
    assert_eq!(&indents[..4], &[0, 2, 2, 0]);
}

#[test]
fn a_string_over_several_lines_keeps_later_lines_empty() {
    let words = words("x = \"a\nFoo\nb\"\nBar\n");
    assert_eq!(words[1], Vec::<String>::new());
    assert_eq!(words[3], vec!["Bar"]);
}

#[test]
fn a_regex_may_open_a_line() {
    let src = "def matcher\n  /\n    \\A(A)\n  /ix\nend\nAfter\n";
    assert_eq!(all_words(src), vec!["def", "matcher", "end", "After"]);
}

#[test]
fn operator_symbols_are_not_regexes() {
    let src = "super(:/, left, right)\nsend(:[]=, k)\nk /= 2\nAfter\n";
    assert_eq!(
        all_words(src),
        vec!["super", "left", "right", "send", "k", "k", "After"]
    );
}

#[test]
fn method_names_ending_in_marks() {
    assert_eq!(
        all_words("a.empty? && b.save!\n"),
        vec!["a", "empty?", "b", "save!"]
    );
}
