use tspp_dir as dir;
use tspp_repository::ArtifactAttemptRecorder;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::CheckState;

/// One symbol's settled static value.
enum StaticValue {
    /// A scalar literal.
    Literal(dir::Literal),
    /// A static term already interned.
    Term(dir::GlobalStaticId),
}

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::sema) fn write_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        // seal typing positions, writeback projects declared forms fresh
        self.infer.seal();

        // collect the literal value each symbol resolved to
        let recorder = self.recorder;
        let symbol_literals =
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.literals", || {
                self.static_symbol_literals(module)
            })?;

        // write symbol values as final statics, a literal pushed and a static term named
        let state = self.module_mut(module);
        for (symbol, value) in symbol_literals {
            let id = match value {
                StaticValue::Literal(literal) => state
                    .statics_tail
                    .push_static(dir::StaticTerm::Literal { value: literal })
                    .into_global(module),
                StaticValue::Term(id) => id,
            };
            state.statics_tail.set_symbol_static(symbol, id);
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

        // evaluate module constants the check phase left undecided
        let constants =
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.constants", || {
                self.static_module_constants(module)
            })?;
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
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.captures", || {
            self.write_captures(module)
        })?;

        // settle the member sites and narrowings on the pass's final solution
        self.resolve_member_subjects(module)?;
        if !self.is_declaring() {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.narrowings", || {
                self.settle_narrowings(module)
            })?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "write.members", || {
                self.settle_member_resolutions(module)
            })?;
        }

        Ok(())
    }

    /// Evaluate module const initializers into static terms after solving.
    fn static_module_constants(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::StaticTerm)>> {
        // read the module's expanded tree
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        // collect the constant declarator bindings and the associated consts first
        let mut bindings = Vec::new();
        for root in &expanded.roots {
            match *tree.get(*root) {
                dir::Expression::Let {
                    mutability: dir::Mutability::Immutable,
                    ref declarators,
                    ..
                } => {
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
                dir::Expression::Declaration(declaration) => {
                    for member in tree.get(declaration).member_ids().into_iter().flatten() {
                        let dir::Member::AssociatedConst {
                            value: Some(value), ..
                        } = *tree.get(*member)
                        else {
                            continue;
                        };
                        let Some(symbol) =
                            self.module(module).declaration_symbol(member.into_any())
                        else {
                            continue;
                        };
                        bindings.push((symbol, value));
                    }
                }
                _ => {}
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

    /// Settle one module's symbol values: the literals and the static terms their types hold.
    fn static_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, StaticValue)>> {
        // collect the values the walk stored per symbol
        let static_values = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();

        // keep the symbols whose value resolved to a scalar literal or a static term
        let mut literals = Vec::new();
        for (symbol, value) in static_values {
            let value = self.shallow_resolve(value)?;
            match self.ty(value)? {
                dir::Type::Literal(literal) => {
                    literals.push((symbol, StaticValue::Literal(literal)))
                }
                dir::Type::Static(id) => literals.push((symbol, StaticValue::Term(id))),
                _ => {}
            }
        }

        Ok(literals)
    }
}
