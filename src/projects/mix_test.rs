use super::*;

fn dep(target: DepTarget, scope: Scope) -> RawDep {
    RawDep { target, scope }
}

fn path(p: &str) -> DepTarget {
    DepTarget::Path(PathBuf::from(p))
}

const MIX: &str = r#"
defmodule Invoicing.MixProject do
  use Mix.Project

  def project do
    [
      app: :invoicing,
      version: "0.1.0",
      elixirc_paths: elixirc_paths(Mix.env()),
      build_path: "_build",
      deps: deps()
    ]
  end

  defp deps do
    [
      {:phoenix, "~> 1.7"},
      {:billing_core, path: "../../libs/billing_core"},
      # {:old_lib, path: "../../libs/old_lib"},
      {:test_helpers, path: "../../libs/test_helpers", only: :test},
      {
        :tooling,
        path: "../../libs/tooling", # shared mix tasks
        only: [:dev, :test],
        runtime: false
      },
      {:metrics, path: "../../libs/metrics", runtime: false},
      {:pdf, path: "../../libs/pdf", optional: true},
      {:obs, path: "../../libs/obs", only: [:prod, :staging]},
      {:remote, git: "https://example.com/remote.git"}
    ]
  end
end
"#;

#[test]
fn reads_the_app_name_and_its_path_dependencies() {
    let m = read(MIX);

    assert_eq!(m.name.as_deref(), Some("invoicing"));
    assert_eq!(
        m.deps,
        [
            dep(path("../../libs/billing_core"), Scope::Runtime),
            dep(path("../../libs/test_helpers"), Scope::Dev),
            dep(path("../../libs/tooling"), Scope::Dev),
            dep(path("../../libs/metrics"), Scope::Build),
            dep(path("../../libs/pdf"), Scope::Optional),
            dep(path("../../libs/obs"), Scope::Runtime),
        ]
    );
    assert_eq!(m.unread, 0);
}

#[test]
fn umbrella_dependencies_are_resolved_by_name() {
    let m = read(
        r#"
  def project, do: [app: :web, deps: deps()]
  defp deps do
    [{:core, in_umbrella: true}, {:fixtures, in_umbrella: true, only: :test}]
  end
"#,
    );
    assert_eq!(
        m.deps,
        [
            dep(DepTarget::Name("core".into()), Scope::Runtime),
            dep(DepTarget::Name("fixtures".into()), Scope::Dev),
        ]
    );
}

#[test]
fn umbrella_root_is_not_a_project() {
    let m = read(
        r#"
  def project do
    [apps_path: "apps", version: "0.1.0", deps: []]
  end
"#,
    );
    assert_eq!(m.name, None);
    assert!(m.deps.is_empty());
    assert_eq!(m.unread, 0);
}

#[test]
fn paths_built_at_run_time_are_counted_as_unread() {
    let m = read(
        r#"
  def project, do: [app: :web, deps: deps()]
  defp deps do
    [
      {:core, path: "../core"},
      {:shared, path: Path.expand("../shared", __DIR__)},
      {:named, path: "../#{@prefix}_named"}
    ] ++ Enum.map(@libs, fn lib -> {lib, path: "../libs/#{lib}"} end)
  end
"#,
    );
    assert_eq!(m.deps, [dep(path("../core"), Scope::Runtime)]);
    // Path.expand, the interpolated literal, and the generated tuple.
    assert_eq!(m.unread, 3);
}

#[test]
fn a_hash_inside_a_string_or_a_char_literal_is_not_a_comment() {
    let m = read(
        r#"
  def project, do: [app: :web, description: "issue #12", deps: deps()]
  defp deps do
    [{:core, path: "../core#x"}, {:sep, path: sep(?#, "../sep")}, {:late, path: "../late"}]
  end
"#,
    );
    assert_eq!(
        m.deps,
        [
            dep(path("../core#x"), Scope::Runtime),
            dep(path("../late"), Scope::Runtime),
        ]
    );
    assert_eq!(m.unread, 1);
}

#[test]
fn keys_are_matched_as_whole_words() {
    let m = read(
        r#"
  def project, do: [app: :web, source_path: "src", my_app: :other, deps: deps()]
  defp deps, do: [{:core, sub_path: "../nope"}, {:real, path: "../real"}]
"#,
    );
    assert_eq!(m.name.as_deref(), Some("web"));
    assert_eq!(m.deps, [dep(path("../real"), Scope::Runtime)]);
    assert_eq!(m.unread, 0);
}

#[test]
fn an_empty_file_declares_nothing() {
    assert_eq!(read(""), Manifest::default());
}
