use destack_dir::{GlobalNodeIdAny, GlobalSymbolId, SymbolTable, SymbolType};
use destack_workspace::{Module, ProfileId};

use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    pub(super) fn resolve_canonical_symbol_chain(
        &self,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        start_symbol: GlobalSymbolId,
        current_symbols: &SymbolTable,
    ) -> ResolveResult<GlobalSymbolId> {
        // track the original calling module (from node)
        // (we don't need to require_task for this module since we're being called DURING its resolution)
        let calling_module = node.module_id;
        let mut current = start_symbol;
        let mut visited = Vec::new();
        loop {
            // detect cyclic symbol reference
            if visited.contains(&current) {
                return Err(ResolveError::CyclicSymbol {
                    node: node.into_anchored(Some(profile_id)),
                    symbol: start_symbol,
                });
            }
            visited.push(current);

            // ensure the target module's direct symbols are resolved (may yield) - skip if it's the calling module
            self.require_dir_prepared_if_other(calling_module, current.module_id, profile_id)?;

            // read the symbol state
            let (symbol_type, canonical_symbol, target_symbol) =
                if current.module_id == calling_module {
                    let symbol = current_symbols.get_symbol(current.local_id);
                    (symbol.ty, symbol.canonical_symbol, symbol.target_symbol)
                } else {
                    let dir = self
                        .require_artifact_dir_prepared(current.module_id, profile_id)
                        .map_err(ResolveError::from)?;
                    let symbol = dir.symbols.get_symbol(current.local_id);
                    (symbol.ty, symbol.canonical_symbol, symbol.target_symbol)
                };

            // stop alias resolution for nominal newtypes
            if symbol_type == SymbolType::Newtype {
                return Ok(current);
            }

            // if symbol already has canonical_symbol computed, use it (optimization)
            if let Some(canonical_symbol) = canonical_symbol {
                return Ok(canonical_symbol);
            }

            // if it has a target, follow it
            if let Some(target) = target_symbol {
                current = target;
            } else {
                // no more targets, this is the final symbol
                return Ok(current);
            }
        }
    }

    /// Resolve and set the canonical_symbol for a symbol that has a target_symbol.
    pub(crate) fn resolve_canonical_symbol(
        &self,
        _module: &Module,
        symbols: &mut SymbolTable,
        node: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        profile: ProfileId,
    ) -> ResolveResult<GlobalSymbolId> {
        // get the target_symbol
        let symbol = symbols.get_symbol(symbol_id.local_id);
        if symbol.ty == SymbolType::Newtype {
            return Ok(symbol_id);
        }
        let Some(target_symbol) = symbol.target_symbol else {
            // no target, this symbol is its own final
            return Ok(symbol_id);
        };
        // follow the chain from target
        let canonical_symbol =
            self.resolve_canonical_symbol_chain(profile, node, target_symbol, symbols)?;

        // set the canonical_symbol
        symbols.get_symbol_mut(symbol_id.local_id).canonical_symbol = Some(canonical_symbol);

        Ok(canonical_symbol)
    }
}
