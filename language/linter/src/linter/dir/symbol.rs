use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the declaration node that introduced one checked symbol.
    pub fn symbol_declaration(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<dir::GlobalNodeIdAny, ProviderError> {
        if symbol.module_id != self.id {
            return Err(ProviderError::internal(format!(
                "symbol {symbol:?} belongs to another module"
            )));
        }

        // read the checked symbol declaration
        let binding = self
            .bindings
            .get_symbol_maybe(symbol.local_id)
            .ok_or_else(|| ProviderError::internal(format!("missing checked symbol {symbol:?}")))?;
        let declaration = binding.declaration.ok_or_else(|| {
            ProviderError::internal(format!("checked symbol {symbol:?} has no declaration"))
        })?;

        Ok(declaration)
    }

    /// Return the symbol introduced by one checked declaration node.
    pub fn declaration_symbol<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> Result<dir::GlobalSymbolId, ProviderError> {
        // read the checked declaration binding
        let declaration = node.into_global_any(self.id);
        let symbol = self
            .bindings
            .declaration_symbol(declaration)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked declaration {declaration:?} introduces no symbol"
                ))
            })?;

        Ok(symbol.into_global(self.id))
    }

    /// Return the single declaration symbol selected directly by one checked expression.
    pub fn selected_symbol(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        // select the checked resolution for supported expression forms
        let view = self.view();
        let global = node.into_global_any(self.id);
        let symbol = match view.get(node) {
            dir::Expression::Identifier { .. } => {
                let resolution = self.resolutions.name_resolution(global).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "checked identifier expression {} in module {:?} has no name resolution",
                        node.id, self.id
                    ))
                })?;
                let [symbol] = resolution.symbols() else {
                    return Ok(None);
                };

                *symbol
            }
            dir::Expression::Member { .. } => {
                let Some(resolution) = self.member_decision(node)? else {
                    return Ok(None);
                };
                let dir::OperationResolution::One(access) = resolution else {
                    return Ok(None);
                };
                let Some(symbol) = access.target.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            _ => return Ok(None),
        };

        Ok(Some(symbol))
    }
}
