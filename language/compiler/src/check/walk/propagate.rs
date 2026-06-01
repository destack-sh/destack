use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Condition, GenericArgument, GenericSlot, Origin, TypeOperand, TypeTerm,
};

impl CheckState<'_> {
    /// Propagate walk-owned state before solving constraints.
    pub(in crate::check) fn propagate_walk_state(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        // induce hidden owner generics before declaration self references
        self.induce_generics()?;

        // equate declarations after all generic slots are fixed
        for module in modules {
            self.equate_declaration_self_types(*module);
        }

        Ok(())
    }

    /// Equate declaration type variables after generic slots are fixed.
    pub(in crate::check) fn equate_declaration_self_types(&mut self, module: ModuleId) {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| self.symbol_has_declaration_type(module, *symbol))
            .collect::<Vec<_>>();

        // include every explicit and induced slot
        for symbol in symbols {
            self.equate_declaration_self_type(module, symbol);
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

    /// Equate one declaration type variable to its self reference.
    fn equate_declaration_self_type(&mut self, module: ModuleId, symbol: dir::GlobalSymbolId) {
        let mut parameters = self
            .generic_slots_for_owner(module, symbol)
            .map(|(variable, generic)| (generic.slot().index, variable, generic.clone()))
            .collect::<Vec<_>>();
        parameters.sort_by_key(|(index, _, _)| *index);

        let arguments = parameters
            .into_iter()
            .map(|(_, variable, generic)| match generic {
                // <T>
                GenericSlot::Type { .. } => GenericArgument::Type(variable.into()),
                // <...T>
                GenericSlot::VariadicType { .. } => GenericArgument::SpreadType(variable.into()),
                // <comptime C: T>
                GenericSlot::Static { .. } => GenericArgument::Static(variable.into()),
                // <comptime ...C: T>
                GenericSlot::VariadicStatic { .. } => {
                    GenericArgument::SpreadStatic(variable.into())
                }
            })
            .collect();

        let term = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments,
        };

        match self.require_symbol_type(symbol) {
            TypeOperand::Variable(variable) => {
                self.equate_type(variable, term, Condition::Always);
            }
            TypeOperand::Term(_) => {
                panic!("check declaration symbol {symbol:?} already has checked type term")
            }
            TypeOperand::Type(_) => {}
        }
    }
}
