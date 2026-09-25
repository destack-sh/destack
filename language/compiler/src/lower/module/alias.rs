use tspp_dir as dir;

use crate::CompilerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Return the value one type alias names, none for every other declaration.
    pub(in crate::lower) fn alias_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(dir::Definition::TypeAlias(_)) = self.definition(symbol)? else {
            return Ok(None);
        };

        // read the value the alias names
        let value = self.symbol_type(symbol)?;

        Ok(Some(value))
    }
}
