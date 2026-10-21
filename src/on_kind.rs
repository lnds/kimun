//! Commands that read the files of one kind, whatever `--kind` says:
//! `km docs` reads documents, `km infra` infrastructure.

use std::error::Error;

use crate::cli::{CommonArgs, OutputMode};
use crate::config::KimunConfig;
use crate::kind::{Kind, Selection};
use crate::walk::WalkConfig;

/// Run `f` over a walk of the files of `kind`, with the exclusions of the
/// command line and of `.kimun.toml`.
pub fn run(
    kind: Kind,
    common: CommonArgs,
    f: impl FnOnce(&WalkConfig<'_>, OutputMode) -> Result<(), Box<dyn Error>>,
) {
    let config = KimunConfig::load();
    let filter = common
        .exclude_args
        .exclude_filter_with(&config.exclude)
        .with_kinds(Selection::new(&[kind], &config.kinds));
    let include_tests = common.include_tests;
    crate::maybe_list_excluded(&common.path, include_tests, &filter, common.list_excluded());
    crate::run_command(common.path, |t| {
        f(&WalkConfig::new(t, include_tests, &filter), common.format)
    });
}
