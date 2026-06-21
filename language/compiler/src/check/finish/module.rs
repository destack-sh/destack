use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    Answer, CheckError, CheckState, Condition, Constraint, ConstraintId, Decision, Origin, Relation,
};
use crate::{CompilerError, CompilerResult};

use super::CheckModuleOutput;

/// One module's final checked rows.
pub(in crate::check) struct ModuleRows {
    /// The module that owns these rows.
    module: ModuleId,
    /// Variable entries patched with their solution types.
    patches: Vec<(dir::LocalTypeId, dir::Type)>,
    /// Resolved node types.
    node_types: Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>,
    /// Resolved symbol types.
    symbol_types: Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>,
    /// Resolved alias definition values.
    definition_values: Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>,
    /// Resolved literal symbol values.
    symbol_literals: Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>,
    /// Resolved implicit coercions.
    coercions: Vec<(dir::GlobalNodeIdAny, dir::Coercion)>,
}

impl ModuleRows {
    /// Return the module that owns these rows.
    pub(in crate::check) fn module(&self) -> ModuleId {
        self.module
    }
}

impl CheckState<'_> {
    /// Read one module's final rows against live working state.
    pub(in crate::check) fn module_rows(&mut self, module: ModuleId) -> CompilerResult<ModuleRows> {
        let (contextual, coercions) = self.derive_node_rows(module)?;

        Ok(ModuleRows {
            module,
            patches: self.finish_variable_patches(module)?,
            node_types: self.finish_node_types(module, &contextual)?,
            symbol_types: self.finish_symbol_types(module)?,
            definition_values: self.finish_definition_values(module)?,
            symbol_literals: self.finish_symbol_literals(module)?,
            coercions,
        })
    }

    /// Finish one module's solved state into output DIR tables.
    pub(in crate::check) fn finish_module(
        &mut self,
        rows: ModuleRows,
    ) -> CompilerResult<CheckModuleOutput> {
        let module = rows.module;
        let mut output = CheckModuleOutput::new(module, self.module(module));

        // move the open overlays out into the final output
        let state = self.module_mut(module);
        output.types = state.take_types();
        output.definitions = state.take_definitions();
        output.generics = state.take_generics();
        if let Some(layouts) = self.layouts.swap_remove(&module) {
            output.layouts = layouts;
        }

        // patch open variable entries with their solution types
        for (local, patched) in rows.patches {
            output.types.update_type(local, patched);
        }

        // alias definition values finish evaluated like their symbol types
        for (symbol, value) in rows.definition_values {
            if let Some(dir::Definition::TypeAlias(definition)) =
                output.definitions.definition_mut(symbol)
            {
                definition.value = value;
            }
        }

        // record inferred node and symbol types
        for (node, ty) in rows.node_types {
            output.types.set_node_type(node, ty);
        }
        for (symbol, ty) in rows.symbol_types {
            output.types.set_symbol_type(symbol, ty);
        }

        // record implicit coercions beside their value nodes
        for (node, coercion) in rows.coercions {
            output.coercions.bind_coercion(node, coercion);
        }

        // symbol values materialize as final statics
        for (symbol, literal) in rows.symbol_literals {
            let id = output
                .statics
                .push_static(dir::StaticTerm::ScalarLiteral { value: literal });
            output
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // drain decided node meanings into resolutions
        self.drain_decisions(module, &mut output);

        // TODO #Incomplete: synthesize capture frames from collected captures

        Ok(output)
    }

    /// Resolve every solved variable entry in one live working segment.
    /// Unsolved variables patch to the error type, their diagnostics
    /// come from the unsolved sweep.
    fn finish_variable_patches(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::LocalTypeId, dir::Type)>> {
        // collect the variable entries owned by this segment
        let working = &self.module(module).types;
        let mut variables = Vec::new();
        for local in working.iter_type_ids() {
            let Some(dir::Type::Variable(variable)) = working.get_type_maybe(local) else {
                continue;
            };

            variables.push((local, *variable));
        }

        // resolve each entry to its solution's type
        let mut patches = Vec::with_capacity(variables.len());
        let mut reported = IndexSet::new();
        for (local, variable) in variables {
            let representative = self.variables.representative(variable)?;
            let solution = self.variables.solution(representative)?;
            let patched = match solution {
                Some(solution) => {
                    let solved = self.resolve_shallow(solution)?;

                    self.ty(solved)?.clone()
                }
                // rejected nodes already reported the primary error
                None => {
                    let origin = self.variables.get(representative)?.origin;
                    if !self.origin_has_rejected_decision(origin) {
                        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                        if self.modules.contains_key(&module)
                            && reported.insert((module, anchor.clone()))
                        {
                            let error = CheckError::MissingTypeAnnotation { anchor, module };
                            self.module_mut(module).diagnostics.push(error.into());
                        }
                    }

                    dir::Type::Error
                }
            };
            patches.push((local, patched));
        }

        Ok(patches)
    }

    /// Return whether one origin already has a primary rejection diagnostic.
    fn origin_has_rejected_decision(&self, origin: Origin) -> bool {
        let Origin::Node(node) = origin else {
            return false;
        };

        matches!(self.decisions.get(node), Some(Decision::Rejected))
    }

    /// Resolve one module's recorded node types.
    fn finish_node_types(
        &mut self,
        module: ModuleId,
        contextual: &IndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>> {
        let node_types = self.module(module).types.node_types().collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(node_types.len());
        for (node, ty) in node_types {
            let ty = contextual
                .get(&node)
                .copied()
                .unwrap_or(self.resolve_shallow(ty)?);
            resolved.push((node, ty));
        }

        Ok(resolved)
    }

    /// Resolve one finished type to its evaluated canonical form.
    /// Finish stores answers: alias applications and preserved type
    /// operations reduce before the tables seal.
    fn finish_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.resolve_shallow(ty)?;
        match self.evaluate_root(origin, ty)? {
            Answer::Ready(evaluated) => Ok(evaluated),
            // unevaluable forms keep their resolved spelling
            Answer::Pending(_) => Ok(ty),
        }
    }

    /// Derive one module's implicit coercions from solved constraints.
    pub(in crate::check) fn derive_coercions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let (_, coercions) = self.derive_node_rows(module)?;

        Ok(coercions)
    }

    /// Derive final node rows from solved assignment constraints.
    fn derive_node_rows(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<(
        IndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
        Vec<(dir::GlobalNodeIdAny, dir::Coercion)>,
    )> {
        let constraints = self
            .constraints
            .iter()
            .map(|(id, constraint)| (id, constraint.clone()))
            .collect::<Vec<_>>();
        let mut contextual = IndexMap::new();
        let mut coercions = IndexMap::new();

        for (id, constraint) in constraints {
            let Some((node, source, target)) =
                self.node_constraint_types(module, id, &constraint)?
            else {
                continue;
            };

            match self.decide_relation(constraint.origin, constraint.relation, source, target)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!("finished node relation is still pending: {blockers:?}"),
                    });
                }
            }

            if self.widens_to(constraint.origin, source, target)? {
                contextual.insert(node, target);
                continue;
            }

            match self.decide_equal(constraint.origin, source, target)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => {
                    let coercion = dir::Coercion::new(source, target, dir::CastOrigin::Implicit);
                    coercions.insert(node, coercion);
                }
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!("finished node relation is still pending: {blockers:?}"),
                    });
                }
            }
        }

        Ok((contextual, coercions.into_iter().collect()))
    }

    /// Return whether one finished constraint condition is active.
    fn is_active_condition(&mut self, condition: &Condition) -> CompilerResult<bool> {
        match condition {
            Condition::Always => Ok(true),
            Condition::When(predicates) => match self.decide_condition(predicates)? {
                Answer::Ready(is_active) => Ok(is_active),
                Answer::Pending(blockers) => Err(CompilerError::Internal {
                    message: format!(
                        "finished constraint condition is still pending: {blockers:?}"
                    ),
                }),
            },
        }
    }

    /// Return node, source, and target types for one completed node constraint.
    fn node_constraint_types(
        &mut self,
        module: ModuleId,
        id: ConstraintId,
        constraint: &Constraint,
    ) -> CompilerResult<Option<(dir::GlobalNodeIdAny, dir::GlobalTypeId, dir::GlobalTypeId)>> {
        if !self.constraints.is_complete(id) || !self.is_active_condition(&constraint.condition)? {
            return Ok(None);
        }

        let Origin::Node(node) = constraint.origin else {
            return Ok(None);
        };
        if node.module_id != module {
            return Ok(None);
        }
        if !matches!(
            constraint.relation,
            Relation::Assignable | Relation::Writable
        ) {
            return Ok(None);
        }

        let Some(node_type) = self.node_type(node) else {
            return Ok(None);
        };
        let node_type = self.resolve_shallow(node_type)?;
        let source = self.resolve_shallow(constraint.left)?;

        let source = match self.evaluate_root(constraint.origin, source)? {
            Answer::Ready(source) => source,
            Answer::Pending(_) => return Ok(None),
        };

        match self.decide_equal(Origin::Node(node), node_type, source)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) | Answer::Pending(_) => return Ok(None),
        }

        let target = match self.evaluate_root(constraint.origin, constraint.right)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => {
                return Err(CompilerError::Internal {
                    message: format!("finished node target is still pending: {blockers:?}"),
                });
            }
        };

        Ok(Some((node, source, target)))
    }

    /// Resolve one module's recorded symbol types.
    fn finish_symbol_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let symbol_types = self.module(module).types.symbol_types().collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(symbol_types.len());
        for (symbol, ty) in symbol_types {
            // alias values finish their evaluated answer, every other
            // symbol keeps its written spelling for lazy use sites
            let ty = if self.symbol_kind(symbol) == dir::SymbolKind::TypeAlias {
                self.finish_type(Origin::Symbol(symbol), ty)?
            } else {
                self.resolve_shallow(ty)?
            };
            resolved.push((symbol, ty));
        }

        Ok(resolved)
    }

    /// Resolve one module's alias definition values.
    ///
    /// Alias values finish their evaluated answer like alias symbol types.
    fn finish_definition_values(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let aliases = self
            .module(module)
            .definitions
            .iter_definitions()
            .filter_map(|(symbol, definition)| match definition {
                dir::Definition::TypeAlias(definition) => Some((symbol, definition.value)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(aliases.len());
        for (symbol, value) in aliases {
            resolved.push((symbol, self.finish_type(Origin::Symbol(symbol), value)?));
        }

        Ok(resolved)
    }

    /// Resolve one module's literal symbol values.
    fn finish_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>> {
        let symbol_values = self
            .module(module)
            .values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();
        let mut literals = Vec::new();
        for (symbol, value) in symbol_values {
            let value = self.resolve_shallow(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, *literal));
            }
        }

        Ok(literals)
    }

    /// Drain decided node meanings into output resolutions.
    fn drain_decisions(&mut self, module: ModuleId, output: &mut CheckModuleOutput) {
        for (node, decision) in self.decisions.iter() {
            if node.module_id != module {
                continue;
            }

            match decision {
                Decision::Name(resolution) => {
                    output
                        .resolutions
                        .set_name_resolution(node, resolution.clone());
                }
                Decision::Receiver(resolution) => {
                    output
                        .resolutions
                        .set_receiver_resolution(node, *resolution);
                }
                Decision::Member(resolution) => {
                    output
                        .resolutions
                        .set_member_resolution(node, resolution.clone());
                }
                Decision::Call(resolution) => {
                    output
                        .resolutions
                        .set_call_resolution(node, resolution.clone());
                }
                Decision::ReadWrite(resolution) => {
                    output
                        .resolutions
                        .set_read_write_resolution(node, resolution.clone());
                }
                Decision::Construct(resolution) => {
                    output
                        .resolutions
                        .set_construct_resolution(node, resolution.clone());
                }
                Decision::Pattern(resolution) => {
                    output
                        .resolutions
                        .set_pattern_resolution(node, resolution.clone());
                }
                Decision::AssignPattern(resolution) => {
                    output
                        .resolutions
                        .set_assign_pattern_resolution(node, resolution.clone());
                }
                // rejections already carry their diagnostics
                Decision::Rejected => {}
            }
        }
    }
}
