use super::*;

#[test]
fn roles_by_path() {
    let role_of = |p: &str| role(Path::new(p));
    assert_eq!(role_of("CLAUDE.md"), Some(Role::Instructions));
    assert_eq!(role_of("apps/web/AGENTS.md"), Some(Role::Instructions));
    assert_eq!(role_of(".cursorrules"), Some(Role::Rules));
    assert_eq!(role_of(".cursor/rules/rust.mdc"), Some(Role::Rules));
    assert_eq!(
        role_of(".github/copilot-instructions.md"),
        Some(Role::Rules)
    );
    assert_eq!(role_of(".claude/skills/deploy/SKILL.md"), Some(Role::Skill));
    assert_eq!(role_of(".claude/agents/reviewer.md"), Some(Role::Agent));
    assert_eq!(role_of(".claude/commands/ship.md"), Some(Role::Command));
    assert_eq!(role_of("README.md"), None);
    assert_eq!(role_of(".claude/settings.json"), None);
}

#[test]
fn tokens_are_about_four_characters() {
    assert_eq!(tokens(""), 0);
    assert_eq!(tokens("abcd"), 1);
    assert_eq!(tokens("abcde"), 2);
}

#[test]
fn rules_are_lines_that_ask_outside_code() {
    let text = "# Rules\n\
                - Always run tests.\n\
                - You MUST NOT push.\n\
                - Nunca uses voseo.\n\
                - Mustard is a sauce.\n\
                ```\nnever here\n```\n\
                Plain line.\n";
    assert_eq!(rules(text), 3);
}

#[test]
fn repeated_lines_across_files() {
    let a = "- Run cargo fmt before cargo clippy, always.\nOnly in a, long enough to count here.\n";
    let b = "Intro\n* **Run cargo fmt before cargo clippy, always.**\n";
    let c =
        "Run cargo fmt before cargo clippy, always.\nRun cargo fmt before cargo clippy, always.\n";
    let found = repeated(&[a, b, c]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "run cargo fmt before cargo clippy, always.");
    assert_eq!(found[0].places, [(0, 1), (1, 2), (2, 1), (2, 2)]);
    assert!(
        repeated(&[c]).is_empty(),
        "within one file it is no repetition"
    );
}

#[test]
fn a_load_is_the_instructions_from_the_root_down() {
    let files = vec![
        (PathBuf::from("CLAUDE.md"), Role::Instructions, 100),
        (PathBuf::from("apps/web/CLAUDE.md"), Role::Instructions, 50),
        (PathBuf::from("apps/web/AGENTS.md"), Role::Instructions, 20),
        (PathBuf::from("apps/api/CLAUDE.md"), Role::Instructions, 10),
        (PathBuf::from(".claude/skills/x/SKILL.md"), Role::Skill, 500),
    ];
    let loads = loads(&files);
    let summary: Vec<(&str, &str, Vec<usize>, usize)> = loads
        .iter()
        .map(|l| (l.dir.to_str().unwrap(), l.family, l.files.clone(), l.tokens))
        .collect();
    assert_eq!(
        summary,
        [
            ("apps/web", "claude", vec![0, 1], 150),
            ("apps/api", "claude", vec![0, 3], 110),
            ("", "claude", vec![0], 100),
            ("apps/web", "agents", vec![2], 20),
        ]
    );
}
