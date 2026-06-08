use destack_dir as dir;

use crate::core::{DirQueryContext, ExtensionEntry, ModuleQueryContext, WorkspaceQueryContext};

impl ModuleQueryContext<'_> {
    /// Build extension index entries for this module.
    pub(crate) fn build_extension_entries(&self) -> Vec<ExtensionEntry> {
        let mut entries = Vec::new();

        // collect checked extension declarations
        for (extension_symbol, extension) in self.dir().definitions().iter_extensions() {
            let Some(target_symbol) = extension.target.nominal_root() else {
                continue;
            };

            entries.push(ExtensionEntry {
                extension_symbol,
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
        for entry in workspace.extension_entries_for_target(canonical_target) {
            let Some(module_ctx) = self.module_context(entry.extension_symbol.module_id) else {
                continue;
            };

            let definitions = module_ctx.dir().definitions();
            let Some(extension) = definitions.extension_definition(entry.extension_symbol) else {
                continue;
            };

            // filter out not visible extensions
            if !extension.is_visible_from(self.module_id()) {
                continue;
            }

            // stop scanning once the visitor is satisfied
            if visit(module_ctx.dir(), extension) {
                return;
            }
        }
    }
}
