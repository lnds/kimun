use super::*;

#[test]
fn a_file_and_its_test_are_at_the_same_place() {
    let same = |source: &str, test: &str| place(Path::new(source)) == place(Path::new(test));

    assert!(same(
        "lib/app_web/controllers/user_controller.ex",
        "test/app_web/controllers/user_controller_test.exs"
    ));
    assert!(same("apps/a/lib/a/b.ex", "apps/a/test/a/b_test.exs"));
    assert!(same("src/ui/button.ts", "src/ui/button.test.ts"));
    assert!(same("src/ui/button.ts", "src/ui/__tests__/button.spec.ts"));
    assert!(same("pkg/parser.py", "pkg/tests/test_parser.py"));
    assert!(same("src/Parser.hs", "test/ParserSpec.hs"));
    // Another project, another directory or another name is another place.
    assert!(!same("apps/a/lib/a/b.ex", "apps/c/test/a/b_test.exs"));
    assert!(!same(
        "lib/app/users.ex",
        "test/app/accounts/users_test.exs"
    ));
    assert!(!same("lib/app/user.ex", "test/app/users_test.exs"));
}

fn named(test: &str, source: &str) -> bool {
    named_after(&place(Path::new(test)), &place(Path::new(source)))
}

#[test]
fn a_test_named_after_a_file_a_directory_apart() {
    // The source has a directory the test does not.
    assert!(named(
        "test/app_web/page_live_test.exs",
        "lib/app_web/live/page_live.ex"
    ));
    assert!(named(
        "test/app_web/controllers/user_controller_test.exs",
        "lib/app_web/controllers/internal/user_controller.ex"
    ));
    // Or the other way around.
    assert!(named(
        "test/app_web/live/page_live_test.exs",
        "lib/app_web/page_live.ex"
    ));
    // At the same place it is a mirror, not a namesake.
    assert!(!named(
        "test/app_web/page_live_test.exs",
        "lib/app_web/page_live.ex"
    ));
    // Two directories apart, the name alone says little: `helpers` is everywhere.
    assert!(!named(
        "test/app_web/helpers_test.exs",
        "lib/app_web/live/users_live/helpers.ex"
    ));
    // Another branch of the tree is another file.
    assert!(!named(
        "test/app_web/admin/page_live_test.exs",
        "lib/app_web/live/page_live.ex"
    ));
    assert!(!named(
        "test/other_web/page_live_test.exs",
        "lib/app_web/live/page_live.ex"
    ));
}

#[test]
fn a_test_named_after_the_directory_of_a_file() {
    // A view with a directory of its own, tested as a whole.
    assert!(named(
        "test/app_web/live/operators_live_test.exs",
        "lib/app_web/live/operators_live/index.ex"
    ));
    assert!(named(
        "test/app_web/operators_live_test.exs",
        "lib/app_web/live/operators_live/form.ex"
    ));
    // The directory has to be the one right above the file.
    assert!(!named(
        "test/app_web/live/operators_live_test.exs",
        "lib/app_web/live/operators_live/components/row.ex"
    ));
    assert!(!named(
        "test/app_web/live/users_live_test.exs",
        "lib/app_web/live/operators_live/index.ex"
    ));
    assert!(!named("test/x_test.exs", "x.ex"));
}

/// Tests of the files given as `(path, is_test)`, with `refers` as the
/// pairs `(test, file)` of what each test refers to, by index.
fn tests_of(files: &[(&str, bool)], refers: &[(usize, usize)]) -> Tests {
    let paths: Vec<PathBuf> = files.iter().map(|(p, _)| PathBuf::from(p)).collect();
    let mut referring = vec![Vec::new(); files.len()];
    for &(test, file) in refers {
        referring[file].push(test);
    }
    Tests::of(&paths, referring, |f| files[f].1, |f| !files[f].1)
}

#[test]
fn protection_from_the_strongest_test_there_is() {
    let files = [
        ("lib/app/users.ex", false), // 0: a test refers to it
        ("lib/app_web/controllers/user_controller.ex", false), // 1: mirrored
        ("lib/app_web/live/page_live.ex", false), // 2: named
        ("lib/app/formatting.ex", false), // 3: used by 2
        ("lib/app/orphan.ex", false), // 4: nothing
        ("lib/app/leaf.ex", false),  // 5: used by 4 only
        ("test/app/accounts_test.exs", true), // 6
        ("test/app_web/controllers/user_controller_test.exs", true), // 7
        ("test/app_web/page_live_test.exs", true), // 8
    ];
    let tests = tests_of(&files, &[(6, 0)]);

    assert_eq!(tests.protection(0, &[]), Protection::Direct);
    assert_eq!(tests.direct(0), [6]);
    assert_eq!(tests.protection(1, &[]), Protection::Direct);
    assert_eq!(tests.direct(1), [7]);
    assert_eq!(tests.protection(2, &[]), Protection::Named);
    assert!(tests.direct(2).is_empty());
    assert_eq!(tests.own(2).collect::<Vec<_>>(), [8]);
    // No test of its own; the view that uses it has one.
    assert_eq!(tests.protection(3, &[2]), Protection::Users);
    assert_eq!(tests.protection(4, &[]), Protection::None);
    // Being used by a file with no test protects nothing.
    assert_eq!(tests.protection(5, &[4]), Protection::None);
    // A user that is itself only protected through its users does not count.
    assert_eq!(tests.protection(5, &[3]), Protection::None);
}

#[test]
fn a_reference_and_a_mirror_count_once() {
    let files = [
        ("lib/app/users.ex", false),
        ("test/app/users_test.exs", true),
    ];
    let tests = tests_of(&files, &[(1, 0)]);
    assert_eq!(tests.direct(0), [1]);
}

#[test]
fn protection_orders_from_strongest_to_none() {
    assert!(Protection::Direct < Protection::Named);
    assert!(Protection::Named < Protection::Users);
    assert!(Protection::Users < Protection::None);
}
