use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, GenericArgument, Origin, StaticTerm, TypeOperand, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

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
            self.equate_declaration_self_types(*module)?;
        }

        Ok(())
    }

    /// Equate declaration type variables after generic slots are fixed.
    pub(in crate::check) fn equate_declaration_self_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| self.symbol_has_declaration_type(*symbol))
            .collect::<Vec<_>>();

        // include every explicit and induced slot
        for symbol in symbols {
            self.equate_declaration_self_type(symbol)?;
        }

        Ok(())
    }

    /// Return whether one symbol owns a declaration type.
    fn symbol_has_declaration_type(&self, symbol: dir::GlobalSymbolId) -> bool {
        matches!(
            self.symbol_kind(symbol),
            dir::SymbolKind::Class
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Interface
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Newtype
                | dir::SymbolKind::NewtypeInterface
        )
    }

    /// Equate one declaration type variable to its self reference.
    fn equate_declaration_self_type(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<()> {
        let parameters = self
            .inference
            .generic_parameters_for_owner(symbol)
            .map(|(slot, generic)| (slot, generic.is_static(), generic.is_variadic()))
            .collect::<Vec<_>>();

        let arguments = parameters
            .into_iter()
            .map(|(slot, is_static, is_variadic)| {
                if is_static {
                    let parameter = self.inference.push_term(StaticTerm::Parameter(slot));

                    if is_variadic {
                        GenericArgument::SpreadStatic(parameter.into())
                    } else {
                        GenericArgument::Static(parameter.into())
                    }
                } else {
                    let parameter = self.inference.push_term(TypeTerm::Parameter(slot));

                    if is_variadic {
                        GenericArgument::SpreadType(parameter.into())
                    } else {
                        GenericArgument::Type(parameter.into())
                    }
                }
            })
            .collect();

        let term = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments,
        };

        match self.inputs.symbol_type(symbol) {
            Some(TypeOperand::Variable(variable)) => {
                self.equate_type(variable, term, Condition::Always);
            }
            Some(TypeOperand::Term(_)) => {
                return Err(CompilerError::Internal {
                    message: format!("declaration symbol {symbol:?} already has a type operand"),
                });
            }
            Some(TypeOperand::Type(_)) => {}
            None => {
                return Err(CompilerError::Internal {
                    message: format!("declaration symbol {symbol:?} has no type operand"),
                });
            }
        }

        Ok(())
    }
}
