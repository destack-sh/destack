use destack_dir::{Extension, ExtensionForm, GlobalSymbolId};
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{Repository, Revision};

use crate::core::{
    DirQueryContext, ExtensionEntry, extension_candidates_for_target, query_context,
    query_context_for_profile,
};

use super::get_canonical_symbol;

/// Visit each visible extension that targets the given symbol.
///
/// Returns early when the visitor returns true.
pub(crate) fn for_each_visible_extension(
    repository: &Repository,
    revision: Revision,
    target_symbol: GlobalSymbolId,
    current_module_id: ModuleId,
    mut visit: impl FnMut(DirQueryContext<'_>, &Extension) -> bool,
) {
    // normalize the target symbol across imports and re exports
    let canonical_target = get_canonical_symbol(repository, revision, target_symbol);

    // scan cached extensions for the canonical target
    for entry in extension_candidates_for_target(repository, revision, canonical_target) {
        let Some(ctx) = query_context(repository, revision, entry.module_id) else {
            continue;
        };

        let types = ctx.dir().types();
        let extension = types.get_extension(entry.extension_id);

        // filter out not visible extensions
        if !extension_is_visible(extension, current_module_id) {
            continue;
        }

        // stop scanning once the visitor is satisfied
        if visit(ctx.dir(), extension) {
            return;
        }
    }
}

/// Build extension index entries for one module.
pub(crate) fn build_extension_candidates_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<ExtensionEntry> {
    let Some(ctx) = query_context_for_profile(repository, revision, module_id, profile_id) else {
        return Vec::new();
    };

    let mut entries = Vec::new();

    for (extension_id, extension) in ctx.dir().types().iter_extensions() {
        entries.push(ExtensionEntry {
            module_id,
            extension_id,
            target_symbol: get_canonical_symbol(repository, revision, extension.target),
        });
    }

    entries
}

/// Check whether an extension is visible from a module.
fn extension_is_visible(extension: &Extension, current_module_id: ModuleId) -> bool {
    // choose visibility rules by extension form
    match extension.form {
        ExtensionForm::Inherent => true,
        ExtensionForm::Local => extension.symbol.module_id == current_module_id,
        ExtensionForm::Named => true,
    }
}
