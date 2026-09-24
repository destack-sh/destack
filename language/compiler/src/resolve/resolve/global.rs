use destack_artifact::{EnvironmentBound, LanguageEnvironment};
use destack_dir as dir;

use crate::resolve::state::ResolveState;
use crate::{CompilerError, CompilerResult};

impl ResolveState<'_> {
    /// Resolve language items used by active roots.
    ///
    /// Example:
    /// ```ds
    /// async function load() {
    ///     await task;
    /// }
    /// // `async` uses Promise without naming it
    /// ```
    pub(in crate::resolve) fn resolve_language_items(
        &mut self,
        language: &LanguageEnvironment,
    ) -> CompilerResult<()> {
        let items = self.language_items.iter().copied().collect::<Vec<_>>();

        for item in items {
            self.resolve_language_item_use(language, item)?;
        }

        Ok(())
    }

    /// Resolve one used language item to its symbol.
    ///
    /// Example:
    /// ```ds
    /// const value = first + second;
    /// // `+` uses Add
    /// ```
    fn resolve_language_item_use(
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
        environment: &EnvironmentBound,
    ) -> CompilerResult<()> {
        let keys = self.global_keys.iter().copied().collect::<Vec<_>>();

        // read each referenced key from the precomputed table
        for key in keys {
            let Some(resolutions) = environment.global_resolutions_by_key.get(&key) else {
                continue;
            };

            for resolution in resolutions {
                self.imports.push_global_resolution(key, resolution.clone());
            }
        }

        Ok(())
    }
}
