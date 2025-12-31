use destack_builtin::WellKnownSymbol;
use destack_dir::{
    GlobalSymbolId, LocalTypeId, StaticArgument, StaticExpression, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
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

    /// Normalize well-known type references into structural types when possible.
    pub(crate) fn normalize_well_known_type_reference(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        types: &mut TypeTable,
    ) -> Option<Type> {
        let canonical_symbol = self.canonical_symbol_id(module, symbols, profile, symbol);

        // check array reference
        if let Some(array_symbol) = self.get_well_known_symbol(profile, WellKnownSymbol::Array)
            && canonical_symbol == array_symbol
        {
            let element = static_arguments
                .and_then(|arguments| arguments.first())
                .map(|argument| self.static_argument_type(argument, types));
            return Some(Type::Array { element });
        }

        None
    }

    /// Convert a static argument into a type id for type evaluation.
    fn static_argument_type(
        &self,
        argument: &StaticArgument,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let ty = match argument {
            StaticArgument::Evaluated { value, .. } => match value {
                StaticExpression::Type { ty } => return *ty,
                StaticExpression::TypeLiteral { value } => Type::TypeLiteral {
                    value: value.clone(),
                },
                StaticExpression::ScalarLiteral { value } => Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value.clone()),
                },
                _ => Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
            },
            StaticArgument::Unevaluated { .. } => Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
        };

        types.insert_type(ty)
    }
}
