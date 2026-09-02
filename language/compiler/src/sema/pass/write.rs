use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::sema) fn write_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        // seal typing positions, writeback projects declared forms fresh
        self.infer.seal();

        // collect the literal value each symbol resolved to
        let symbol_literals = self.static_symbol_literals(module)?;

        // write symbol values as final statics
        let state = self.module_mut(module);
        for (symbol, literal) in symbol_literals {
            let id = state
                .statics_tail
                .push_static(dir::StaticTerm::Literal { value: literal });
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // collect the identity values
        let identities = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();

        // write each identity value as a type static
        for (symbol, value) in identities {
            let value = self.fully_resolve(value)?;
            let state = self.module_mut(module);
            if state.statics_tail.get_symbol_static_id(symbol).is_some() {
                continue;
            }
            let id = state
                .statics_tail
                .push_static(dir::StaticTerm::Type { ty: value });
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // expose each class declaration as a type static for its value uses
        let classes = self
            .module(module)
            .iter_definitions()
            .filter(|(_, definition)| matches!(definition, dir::Definition::Class(_)))
            .map(|(symbol, _)| symbol)
            .collect::<Vec<_>>();
        for symbol in classes {
            self.class_static_id(symbol)?;
        }

        // evaluate module constants the check phase left undecided
        let constants = self.static_module_constants(module)?;
        let state = self.module_mut(module);
        for (symbol, term) in constants {
            if state.statics_tail.get_symbol_static_id(symbol).is_some() {
                continue;
            }
            let id = state.statics_tail.push_static(term);
            state
                .statics_tail
                .set_symbol_static(symbol, id.into_global(module));
        }

        // write closure capture frames and bindings
        self.write_captures(module)?;

        // settle the member sites and narrowings on the pass's final solution
        if self.is_declaring() {
            self.resolve_member_subjects(module)?;
        } else {
            self.settle_narrowings(module)?;
            self.settle_member_bindings(module)?;
            self.settle_member_resolutions(module)?;
        }

        Ok(())
    }

    /// Evaluate module const initializers into static terms after solving.
    fn static_module_constants(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::StaticTerm)>> {
        // read the module's expanded tree
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        // collect the constant declarator bindings first
        let mut bindings = Vec::new();
        for root in &expanded.roots {
            let dir::Expression::Let {
                mutability: dir::Mutability::Immutable,
                ref declarators,
                ..
            } = *tree.get(*root)
            else {
                continue;
            };

            for declarator in declarators {
                let declarator = tree.get(*declarator);
                let Some(value) = declarator.value else {
                    continue;
                };
                let pattern = declarator.pattern.into_any();
                let Some(symbol) = self.module(module).declaration_symbol(pattern) else {
                    continue;
                };
                bindings.push((symbol, value));
            }
        }

        // evaluate each initializer, keeping only the static ones
        let mut constants = Vec::new();
        for (symbol, value) in bindings {
            if let Ok(term) = self.evaluate_static_expression(module, value)? {
                constants.push((symbol, term));
            }
        }

        Ok(constants)
    }

    /// Settle one module's literal symbol values.
    fn static_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::Literal)>> {
        // collect the values the walk stored per symbol
        let static_values = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();

        // keep the symbols whose value resolved to a scalar literal
        let mut literals = Vec::new();
        for (symbol, value) in static_values {
            let value = self.shallow_resolve(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, literal));
            }
        }

        Ok(literals)
    }
}
