use destack_artifact::{GlobalEnvironment, LanguageEnvironment};
use destack_dir as dir;

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
    /// // console resolves through the profile's global table
    /// ```
    pub(in crate::resolve) fn resolve_profile_globals(
        &mut self,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        let keys = self.global_keys.iter().copied().collect::<Vec<_>>();

        // read each required key from the precomputed table
        for key in keys {
            let Some(targets) = environment.global_targets_by_key.get(&key) else {
                continue;
            };

            for target in targets {
                let module = match target {
                    dir::ImportTarget::Symbol(symbol) => symbol.module_id,
                    dir::ImportTarget::Namespace(module) => *module,
                };

                self.imports.push_module(module);
                self.imports.push_global_target(key, *target);
            }
        }

        Ok(())
    }
}
