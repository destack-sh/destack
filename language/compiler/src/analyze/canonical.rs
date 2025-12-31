use destack_dir::{GlobalSymbolId, SymbolTable};
use destack_workspace::{Module, ProfileId};

use crate::Compiler;

impl Compiler {
    /// Resolve the canonical symbol for a reference.
    pub(crate) fn canonical_symbol_id(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let mut current_symbol = symbol;
        let mut visited = Vec::new();

        // walk target and canonical chains until we stabilize
        loop {
            if visited.contains(&current_symbol) {
                return current_symbol;
            }
            visited.push(current_symbol);

            let (canonical_symbol, target_symbol) = if current_symbol.module_id == module.id {
                let symbol_entry = symbols.get_symbol(current_symbol.local_id);
                (symbol_entry.canonical_symbol, symbol_entry.target_symbol)
            } else {
                let remote_module = self.program.modules.get(current_symbol.module_id);
                let remote_module = remote_module.read();
                let remote_symbols = remote_module.dir(profile).symbols.read();
                let symbol_entry = remote_symbols.get_symbol(current_symbol.local_id);
                (symbol_entry.canonical_symbol, symbol_entry.target_symbol)
            };

            if let Some(canonical_symbol) = canonical_symbol {
                return canonical_symbol;
            }

            if let Some(target_symbol) = target_symbol {
                current_symbol = target_symbol;
            } else {
                return current_symbol;
            }
        }
    }
}
