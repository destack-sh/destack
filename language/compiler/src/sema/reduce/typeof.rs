use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Return the type of a value declaration once its declared type is available.
    pub(in crate::sema) fn reduce_typeof(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // use the same constructor type as an ordinary class reference
        if self.symbol_kind(symbol)? == dir::SymbolKind::Class {
            let ty = self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;

            return Ok(Some(ty));
        }

        // preserve the exact type of a committed constant
        if let Some(ty) = self.static_value(symbol)? {
            return self.shallow_resolve(ty).map(Some);
        }

        // retain a local declaration query until its type has been declared
        if self.is_declaring() && self.adopt_symbol_type_maybe(symbol)?.is_none() {
            return Ok(None);
        }

        let ty = self.symbol_type(symbol)?;
        let ty = self.shallow_resolve(ty)?;

        Ok(Some(ty))
    }
}
