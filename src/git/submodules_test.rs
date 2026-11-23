use super::*;

#[test]
fn the_advice_of_a_submodule_left_out_says_what_to_run() {
    let shown = Path::new("vendor/lib");
    let id = Oid::from_str("8a9b697000000000000000000000000000000000").unwrap();
    let advice = |why: Unreadable| why.advice(shown);
    assert_eq!(
        advice(Unreadable::NotCheckedOut { parent: None }),
        "it is not checked out (git submodule update --init vendor/lib)"
    );
    assert_eq!(
        advice(Unreadable::NotCheckedOut {
            parent: Some(PathBuf::from("vendor"))
        }),
        "it is not checked out (git -C vendor submodule update --init lib)"
    );
    let missing = |shallow, checkout: Option<&str>| Unreadable::MissingCommit {
        id,
        shallow,
        checkout: checkout.map(PathBuf::from),
    };
    assert_eq!(
        advice(missing(true, Some("vendor/lib2"))),
        "commit 8a9b697 is not in its repository (git -C vendor/lib2 fetch --unshallow)"
    );
    assert_eq!(
        advice(missing(false, Some("vendor/lib"))),
        "commit 8a9b697 is not in its repository (git -C vendor/lib fetch)"
    );
    assert_eq!(
        advice(missing(false, None)),
        "commit 8a9b697 is not in the repository git keeps for it"
    );
    assert_eq!(advice(Unreadable::NoUrl), "it has no url in .gitmodules");
    assert_eq!(
        advice(Unreadable::Gone),
        "it was moved or removed since, and git keeps no repository for it"
    );
    assert_eq!(
        advice(Unreadable::NoRepository),
        "no repository holds it (git submodule update --init vendor/lib)"
    );
    assert_eq!(
        advice(Unreadable::Failed("blob missing".to_string())),
        "it could not be read (blob missing)"
    );
}

#[test]
fn a_submodule_name_stays_inside_modules() {
    for name in ["lib", "vendor/lib", "a.b"] {
        assert!(is_submodule_name(name), "{name}");
    }
    for name in ["", "/abs", "..", "a/../b", "./x", r"a\b"] {
        assert!(!is_submodule_name(name), "{name:?}");
    }
}
