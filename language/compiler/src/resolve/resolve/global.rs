use destack_artifact::LanguageEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::resolve::resolve::{ExportLookup, ExportTarget};
use crate::resolve::state::ResolveState;
use crate::{CompilerError, CompilerResult};

impl ResolveState<'_> {
    /// Resolve syntax-required language item symbols.
    ///
    /// Example:
    /// ```ds
    /// async function load() {
    ///     await task;
    /// }
    /// // Promise is required by syntax even when it is not named directly
    /// ```
    pub(in crate::resolve) fn resolve_syntax_language_items(
        &mut self,
        language: &LanguageEnvironment,
    ) -> CompilerResult<()> {
        let items = self.language_items.iter().copied().collect::<Vec<_>>();

        for item in items {
            self.resolve_syntax_language_item(language, item)?;
        }

        Ok(())
    }

    /// Resolve source-visible language globals.
    ///
    /// Example:
    /// ```ds
    /// const value = Array.from(items);
    /// // Array can come from the language environment when no local or profile global wins
    /// ```
    pub(in crate::resolve) fn resolve_language_globals(
        &mut self,
        language: &LanguageEnvironment,
    ) -> CompilerResult<()> {
        let keys = self.global_keys.iter().copied().collect::<Vec<_>>();

        for key in keys {
            self.resolve_language_global(language, key);
        }

        Ok(())
    }

    /// Resolve one source-visible language global when no profile global won.
    ///
    /// Example:
    /// ```ds
    /// const value = String(value);
    /// ```
    fn resolve_language_global(&mut self, language: &LanguageEnvironment, key: dir::StaticKey) {
        if self.imports.global_target_by_key.contains_key(&key) {
            return;
        }
        let dir::StaticKey::Name(name) = key else {
            return;
        };
        let Some(symbol) = language.symbols.get(&name).copied() else {
            return;
        };

        self.imports.push_module(symbol.module_id);
        self.imports
            .push_global_target(key, dir::ImportTarget::Symbol(symbol));
    }

    /// Resolve one syntax-required language item symbol.
    ///
    /// Example:
    /// ```ds
    /// const value = first + second;
    /// // Add is required by operator syntax
    /// ```
    fn resolve_syntax_language_item(
        &mut self,
        language: &LanguageEnvironment,
        item: dir::LanguageItem,
    ) -> CompilerResult<()> {
        let Some(symbol) = language.symbol(item) else {
            return Err(CompilerError::Internal {
                message: format!("missing language item: {item}"),
            });
        };
        self.imports.insert_language_symbol(item, symbol);
        if symbol.module_id == self.module {
            return Ok(());
        }

        self.imports.push_module(symbol.module_id);

        Ok(())
    }

    /// Resolve globals selected by the active profile.
    ///
    /// Example:
    /// ```ds
    /// console.log(value);
    /// // console can be selected from the active profile globals
    /// ```
    pub(in crate::resolve) fn resolve_profile_globals(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        let keys = self.global_keys.iter().copied().collect::<Vec<_>>();
        if keys.is_empty() {
            return Ok(());
        }

        for module in modules {
            self.resolve_global_module_symbols(*module, &keys)?;
        }

        Ok(())
    }

    /// Resolve referenced globals selected from one profile root module.
    ///
    /// Example:
    /// ```ds
    /// document.body;
    /// // document can come from one profile root module
    /// ```
    fn resolve_global_module_symbols(
        &mut self,
        module: ModuleId,
        keys: &[dir::StaticKey],
    ) -> CompilerResult<()> {
        let exported = self
            .artifacts
            .dir_exported(module, self.profile)
            .map_err(CompilerError::from)?;
        self.stats.global_modules += 1;

        for key in keys {
            let Some(entries) = exported.globals.entries_by_key.get(key) else {
                continue;
            };

            for entry in entries {
                match entry {
                    dir::GlobalEntry::Local(entry) => {
                        self.imports.push_module(module);
                        self.imports.push_global_target(
                            *key,
                            dir::ImportTarget::Symbol(entry.source.into_global(module)),
                        );
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
    ///
    /// Example:
    /// ```ds
    /// export { console } from "./console.ds";
    /// // a profile root can re-export the global through another module
    /// ```
    fn resolve_indirect_global_symbol(
        &mut self,
        key: dir::StaticKey,
        entry: &dir::IndirectGlobalEntry,
    ) -> CompilerResult<()> {
        let Some(target) = entry.target else {
            return Ok(());
        };

        if entry.imported == dir::ExportSelector::Namespace {
            self.imports.push_module(target);
            self.imports
                .push_global_target(key, dir::ImportTarget::Namespace(target));

            return Ok(());
        }

        let Some(export_key) = entry.imported.selected_export_key() else {
            return Ok(());
        };

        match self.resolve_export_target(target, export_key)? {
            ExportLookup::Found(ExportTarget::Symbol(symbol)) => {
                self.imports.push_module(symbol.module_id);
                self.imports
                    .push_global_target(key, dir::ImportTarget::Symbol(symbol));
            }
            ExportLookup::Found(ExportTarget::Namespace(module)) => {
                self.imports.push_module(module);
                self.imports
                    .push_global_target(key, dir::ImportTarget::Namespace(module));
            }
            ExportLookup::Ambiguous(_) | ExportLookup::Missing => {}
        }

        Ok(())
    }
}
