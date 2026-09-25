use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, MemoryGrounding, Origin};

impl CheckState<'_> {
    /// Return the type of a value declaration once its declared type is available.
    pub(in crate::sema) fn reduce_typeof(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // use the same constructor type as an ordinary class reference
        if self.symbol_kind(symbol)? == dir::SymbolKind::Class {
            if self.is_declaring() {
                return Ok(None);
            }
            let reference =
                self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;
            let constructors = self.construct_signatures(
                Origin::Symbol(symbol),
                reference,
                MemoryGrounding::Elided,
            )?;
            let mut signatures = Vec::with_capacity(constructors.len());
            for constructor in constructors {
                signatures.push(self.function_type(constructor.ty)?);
            }
            let ty = self.normalized_intersection_type(signatures)?;

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
