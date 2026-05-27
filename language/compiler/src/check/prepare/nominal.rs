use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Definition, GenericArgument, GenericParameter, StaticCondition, TypeTerm,
};

impl CheckState<'_> {
    /// Define nominal declaration type references after generic slots are fixed.
    pub(in crate::check) fn define_nominal_symbol_types(&mut self, module: ModuleId) {
        let binding_table = self.input(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| self.symbol_has_nominal_type(module, *symbol))
            .collect::<Vec<_>>();

        let mut definitions = Vec::with_capacity(symbols.len());

        // build nominal references with every explicit and induced slot
        for symbol in symbols {
            definitions.push(self.nominal_symbol_type_definition(module, symbol));
        }

        // solve nominal references before terms that project from them
        let old = std::mem::take(&mut self.variables.definitions);
        self.variables.definitions = definitions;
        self.variables.definitions.extend(old);
    }

    /// Return whether one symbol owns a nominal type.
    fn symbol_has_nominal_type(&self, module: ModuleId, symbol: dir::GlobalSymbolId) -> bool {
        self.symbol_kind(module, symbol).is_some_and(|kind| {
            matches!(
                kind,
                dir::SymbolKind::Class
                    | dir::SymbolKind::Enum
                    | dir::SymbolKind::Struct
                    | dir::SymbolKind::Newtype
                    | dir::SymbolKind::NewtypeInterface
            )
        })
    }

    /// Return one nominal declaration type reference definition.
    fn nominal_symbol_type_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Definition {
        let variable = self.intern_symbol_type_variable(module, symbol);
        let mut parameters = self
            .generic_parameters_for_owner(symbol)
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
            source: None,
            symbol,
            arguments,
        };
        let term = self.terms.push(term);

        Definition::Type {
            result: variable,
            term,
            origin: self.variable(variable).source,
            condition: StaticCondition::Always,
        }
    }
}
