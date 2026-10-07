//! What Phoenix relates by name alone, where no code says so.

use super::elixir::Reference;

/// The views Phoenix renders a controller with. It finds them by name, so
/// the controller never mentions them: `PageController` goes with
/// `PageHTML` and `PageJSON`, or `PageView`.
pub(super) fn views_of(defines: &[String]) -> impl Iterator<Item = Reference> + '_ {
    defines
        .iter()
        .filter_map(|module| module.strip_suffix("Controller"))
        .filter(|base| !base.is_empty() && !base.ends_with('.'))
        .flat_map(|base| ["HTML", "JSON", "View"].map(|view| format!("{base}{view}")))
        .map(|module| Reference {
            module,
            function: None,
        })
}
