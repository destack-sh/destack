use destack_dir as dir;
use destack_source::ModuleId;

use crate::core::{DirQueryContext, ExtensionEntry, ModuleQueryContext, WorkspaceQueryContext};

impl ModuleQueryContext<'_> {
    /// Build extension index entries for this module.
    pub(crate) fn build_extension_candidates(&self) -> Vec<ExtensionEntry> {
        let mut entries = Vec::new();
        let module_id = self.module_id();

        // collect checked extension declarations
        for (extension_id, extension) in self.dir().extensions().iter_extensions() {
            let Some(target_symbol) = extension.target.nominal_root() else {
                continue;
            };

            entries.push(ExtensionEntry {
                module_id,
                extension_id,
                target_symbol: self.canonical_symbol(target_symbol),
            });
        }

        entries
    }

    /// Visit each visible extension that targets the given symbol.
    ///
    /// Returns early when the visitor returns true.
    pub(crate) fn for_each_visible_extension(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
        mut visit: impl FnMut(DirQueryContext<'_>, &dir::Extension) -> bool,
    ) {
        // normalize the target symbol across imports and re exports
        let canonical_target = self.canonical_symbol(target_symbol);

        // scan cached extensions for the canonical target
        for entry in workspace.extension_candidates_for_target(canonical_target) {
            let Some(module_ctx) = self.module_context(entry.module_id) else {
                continue;
            };

            let extensions = module_ctx.dir().extensions();
            let extension = extensions.get_extension(entry.extension_id);

            // filter out not visible extensions
            if !extension_is_visible(extension, self.module_id()) {
                continue;
            }

            // stop scanning once the visitor is satisfied
            if visit(module_ctx.dir(), extension) {
                return;
            }
        }
    }
}

/// Check whether an extension is visible from a module.
fn extension_is_visible(extension: &dir::Extension, current_module_id: ModuleId) -> bool {
    // choose visibility rules by extension form
    match extension.form {
        dir::ExtensionForm::Inherent => true,
        dir::ExtensionForm::Local => extension.symbol.module_id == current_module_id,
        dir::ExtensionForm::Named => true,
    }
}
