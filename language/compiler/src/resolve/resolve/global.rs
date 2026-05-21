use destack_dir as dir;
use destack_source::ModuleId;

use crate::resolve::resolve::ExportLookup;
use crate::resolve::state::ResolveState;
use crate::{CompilerError, CompilerResult};

impl ResolveState<'_> {
    /// Resolve globals selected by the active profile.
    pub(in crate::resolve) fn resolve_profile_globals(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        for module in modules {
            self.resolve_global_module_symbols(*module)?;
        }

        Ok(())
    }

    /// Resolve globals selected from one profile root module.
    fn resolve_global_module_symbols(&mut self, module: ModuleId) -> CompilerResult<()> {
        let exported = self
            .artifacts
            .dir_exported(module, self.profile)
            .map_err(CompilerError::from)?;

        for (key, entries) in exported.globals.entries() {
            for entry in entries {
                match entry {
                    dir::GlobalEntry::Local(entry) => {
                        self.imports
                            .push_global_symbol(*key, entry.source.into_global(module));
                    }

                    dir::GlobalEntry::Indirect(entry) => {
                        self.resolve_indirect_global_symbol(*key, entry)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Resolve one global re-export through the target module export table.
    fn resolve_indirect_global_symbol(
        &mut self,
        key: dir::StaticKey,
        entry: &dir::IndirectGlobalEntry,
    ) -> CompilerResult<()> {
        let Some(target) = entry.target else {
            return Ok(());
        };
        let Some(export_key) = entry.imported.selected_export_key() else {
            return Ok(());
        };

        if let ExportLookup::Found(symbol) = self.resolve_export_symbol(target, export_key)? {
            self.imports.push_global_symbol(key, symbol);
        }

        Ok(())
    }
}
