use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, Definition, GenericArgument, GenericParameter, Origin, TypeTerm,
};

impl CheckState<'_> {
    /// Add declaration type references after generic slots are fixed.
    pub(in crate::check) fn add_declaration_type_definitions(&mut self, module: ModuleId) {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| self.symbol_has_declaration_type(module, *symbol))
            .collect::<Vec<_>>();

        // define references with every explicit and induced slot
        for symbol in symbols {
            let definition = self.declaration_type_definition(module, symbol);

            self.add_definition(definition);
        }
    }

    /// Return whether one symbol owns a declaration type.
    fn symbol_has_declaration_type(&self, module: ModuleId, symbol: dir::GlobalSymbolId) -> bool {
        self.symbol_kind(module, symbol).is_some_and(|kind| {
            matches!(
                kind,
                dir::SymbolKind::Class
                    | dir::SymbolKind::Enum
                    | dir::SymbolKind::Interface
                    | dir::SymbolKind::Struct
                    | dir::SymbolKind::Newtype
                    | dir::SymbolKind::NewtypeInterface
            )
        })
    }

    /// Return one declaration type reference definition.
    fn declaration_type_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Definition {
        let variable = self.intern_local_symbol_type_variable(module, symbol);
        let mut parameters = self
            .generic_parameters_for_owner(module, symbol)
            .map(|(variable, generic)| (generic.slot().index, variable, generic.clone()))
            .collect::<Vec<_>>();
        parameters.sort_by_key(|(index, _, _)| *index);

        let arguments = parameters
            .into_iter()
            .map(|(_, variable, generic)| match generic {
                // <T>
                GenericParameter::Type { .. } => GenericArgument::Type(variable.into()),
                // <...T>
                GenericParameter::VariadicType { .. } => {
                    GenericArgument::SpreadType(variable.into())
                }
                // <comptime C: T>
                GenericParameter::Static { .. } => GenericArgument::Static(variable.into()),
                // <comptime ...C: T>
                GenericParameter::VariadicStatic { .. } => {
                    GenericArgument::SpreadStatic(variable.into())
                }
            })
            .collect();

        let term = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments,
        };
        let term = self.terms.push(term);

        Definition::Type {
            result: variable,
            term,
            origin: self.variable(variable).source,
            condition: Condition::Always,
        }
    }
}
