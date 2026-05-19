use destack_dir::{Extension, ExtensionForm, GlobalSymbolId};
use destack_source::ModuleId;

use crate::core::{
    DirQueryContext, ExtensionEntry, ModuleQueryContext, WorkspaceQueryContext,
    extension_candidates_for_target,
};

/// Visit each visible extension that targets the given symbol.
///
/// Returns early when the visitor returns true.
pub(crate) fn for_each_visible_extension(
    ctx: &ModuleQueryContext<'_>,
    workspace: &WorkspaceQueryContext<'_>,
    target_symbol: GlobalSymbolId,
    mut visit: impl FnMut(DirQueryContext<'_>, &Extension) -> bool,
) {
    // normalize the target symbol across imports and re exports
    let canonical_target = ctx.canonical_symbol(target_symbol);

    // scan cached extensions for the canonical target
    for entry in extension_candidates_for_target(workspace, canonical_target) {
        let Some(module_ctx) = ctx.module_context(entry.module_id) else {
            continue;
        };

        let extensions = module_ctx.dir().extensions();
        let extension = extensions.get_extension(entry.extension_id);

        // filter out not visible extensions
        if !extension_is_visible(extension, ctx.module_id()) {
            continue;
        }

        // stop scanning once the visitor is satisfied
        if visit(module_ctx.dir(), extension) {
            return;
        }
    }
}

/// Build extension index entries for one module.
pub(crate) fn build_extension_candidates_for_module(
    ctx: &ModuleQueryContext<'_>,
) -> Vec<ExtensionEntry> {
    let mut entries = Vec::new();
    let module_id = ctx.module_id();

    for (extension_id, extension) in ctx.dir().extensions().iter_extensions() {
        entries.push(ExtensionEntry {
            module_id,
            extension_id,
            target_symbol: ctx.canonical_symbol(extension.target),
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
