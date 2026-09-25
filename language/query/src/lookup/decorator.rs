use tspp_dir as dir;

use crate::{ProgramQueryContext, QueryResult};

impl ProgramQueryContext<'_> {
    /// Return whether one exact symbol declaration is deprecated.
    pub(crate) fn symbol_is_deprecated(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<bool> {
        let module = self.module(symbol_id.module_id)?;
        let symbol = module.bindings()?.get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return Ok(false);
        };

        Ok(module
            .decorators()?
            .applications_for_owner(declaration)
            .any(|application| {
                matches!(
                    application.resolution.target,
                    dir::DecoratorTarget::LanguageItem {
                        item: dir::LanguageItem::Deprecated,
                        ..
                    }
                )
            }))
    }
}
