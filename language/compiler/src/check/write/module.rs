use destack_artifact::DiagnosticAnchor;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{
    Answer, CheckState, Constraint, ConstraintId, ConstraintState, Decision, Origin, Relation,
    ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::check) fn write_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let mut reported = IndexSet::new();
        let sealed_variables = self.sealed_variable_types(module)?;
        let node_types = self.resolved_node_types(module, &mut reported)?;
        let symbol_types = self.resolved_symbol_types(module, &mut reported)?;
        let definition_values = self.resolved_definition_values(module, &mut reported)?;
        let symbol_literals = self.static_symbol_literals(module)?;
        let coercions = self.implicit_coercions(module)?;
        if let Some(layouts) = self.layouts.swap_remove(&module) {
            self.module_mut(module).layouts = layouts;
        }

        // seal variable placeholders with their solved types
        let state = self.module_mut(module);
        for (local, sealed) in sealed_variables {
            state.types.update_type(local, sealed);
        }

        // write alias definition values as reduced answers
        for (symbol, value) in definition_values {
            if let Some(dir::Definition::TypeAlias(definition)) =
                state.definitions.definition_mut(symbol)
            {
                definition.value = value;
            }
        }

        // record inferred node and symbol types
        for (node, ty) in node_types {
            state.types.set_node_type(node, ty);
        }
        for (symbol, ty) in symbol_types {
            state.types.set_symbol_type(symbol, ty);
        }

        // record implicit representation changes beside their value nodes
        for (node, coercion) in coercions {
            if let Some(previous) = state.coercions.bind_coercion(node, coercion)
                && previous != coercion
            {
                return Err(CompilerError::Internal {
                    message: format!(
                        "node {node:?} received conflicting coercions {previous:?} and {coercion:?}"
                    ),
                });
            }
        }

        // write symbol values as final statics
        for (symbol, literal) in symbol_literals {
            let id = state
                .statics
                .push_static(dir::StaticTerm::ScalarLiteral { value: literal });
            state
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // drain decided node meanings into resolutions
        self.drain_decisions(module);

        // TODO #Incomplete: synthesize capture frames from collected captures

        Ok(())
    }

    /// Return every variable placeholder with its final written type.
    ///
    /// Unsolved variables write as error because their diagnostics come from the unsolved sweep.
    fn sealed_variable_types(
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
        let mut sealed = Vec::with_capacity(variables.len());
        for (local, variable) in variables {
            let representative = self.solver.representative(variable)?;
            let solution = self.solver.solution(representative)?;
            let ty = match solution {
                Some(solution) => {
                    let solved = self.settled_root(solution)?;

                    self.ty(solved)?.clone()
                }
                None => dir::Type::Error,
            };
            sealed.push((local, ty));
        }

        Ok(sealed)
    }

    /// Return implicit coercions from solved value constraints.
    pub(in crate::check) fn implicit_coercions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let mut constraints = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            let state = self.solver.constraints.state(id)?;
            if state == ConstraintState::Holds && constraint.origin().module() == module {
                constraints.push(id);
            }
        }
        let mut coercions = Vec::new();

        for constraint in constraints {
            if let Some(coercion) = self.constraint_coercion(module, constraint)? {
                coercions.push(coercion);
            }
        }

        Ok(coercions)
    }

    /// Return one implicit coercion from one solved value constraint.
    fn constraint_coercion(
        &mut self,
        module: ModuleId,
        constraint: ConstraintId,
    ) -> CompilerResult<Option<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let (relation, use_, left, right, origin) = {
            let constraint = self.solver.constraints.get(constraint)?;

            let Constraint::Flow(flow) = constraint else {
                return Ok(None);
            };

            (
                flow.relation,
                flow.use_,
                flow.source,
                flow.target,
                flow.origin,
            )
        };
        let Some(node) = origin.expression() else {
            return Ok(None);
        };
        if node.module_id != module {
            return Ok(None);
        }
        if relation != Relation::Assignable {
            return Ok(None);
        }
        if !matches!(
            use_,
            ValueUse::Store | ValueUse::Argument | ValueUse::Output
        ) {
            return Ok(None);
        }

        let source = self.settled_root(left)?;
        let target = self.settled_root(right)?;
        if !self.type_variables(source)?.is_empty() || !self.type_variables(target)?.is_empty() {
            return Ok(None);
        }

        let Some(coercion) = self.implicit_coercion(origin, source, target)? else {
            return Ok(None);
        };

        Ok(Some((node.into_any(), coercion)))
    }

    /// Return the implicit coercion required by one solved source-target pair.
    fn implicit_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Coercion>> {
        if !self.requires_implicit_coercion(origin, source, target)? {
            return Ok(None);
        }

        let coercion = dir::Coercion::new(source, target, dir::CastOrigin::Implicit);

        Ok(Some(coercion))
    }

    /// Resolve one module's recorded node types.
    fn resolved_node_types(
        &mut self,
        module: ModuleId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>> {
        let node_types = self
            .solver
            .node_types()
            .filter_map(|(node, ty)| (node.module_id == module).then_some((node, ty)))
            .collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(node_types.len());
        for (node, ty) in node_types {
            let ty = self.resolved_node_type(node, ty)?;
            self.report_unresolved_output_type(Origin::Node(node), ty, reported)?;
            resolved.push((node, ty));
        }

        Ok(resolved)
    }

    /// Resolve one written node type when reduction is ready.
    fn resolved_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.settled_root(ty)?;
        match self.reduce_type(Origin::Node(node), ty)? {
            Answer::Ready(reduced) => Ok(reduced),
            Answer::Pending(_) => Ok(ty),
        }
    }

    /// Resolve one written type to its reduced canonical form.
    /// Alias applications and preserved type operations reduce before the tables seal.
    fn resolved_output_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.settled_root(ty)?;
        match self.reduce_type(origin, ty)? {
            Answer::Ready(reduced) => Ok(reduced),
            Answer::Pending(blockers) => {
                let (_, anchor) = self.origin_diagnostic_anchor(origin)?;

                Err(CompilerError::Internal {
                    message: format!(
                        "written type for {origin:?} at {anchor:?} is still pending: {blockers:?}"
                    ),
                })
            }
        }
    }

    /// Resolve one module's recorded symbol types.
    fn resolved_symbol_types(
        &mut self,
        module: ModuleId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let mut symbol_types = Vec::new();
        symbol_types.extend(
            self.declaration_types
                .iter()
                .filter_map(|(symbol, ty)| (symbol.module_id == module).then_some((*symbol, *ty))),
        );
        symbol_types.extend(
            self.binding_types
                .iter()
                .filter_map(|(symbol, ty)| (symbol.module_id == module).then_some((*symbol, *ty))),
        );

        let mut resolved = Vec::with_capacity(symbol_types.len());
        for (symbol, ty) in symbol_types {
            // write alias values as reduced answers, every other
            // symbol keeps its written spelling for lazy use sites
            let ty = if self.symbol_kind(symbol) == dir::SymbolKind::TypeAlias {
                self.resolved_output_type(Origin::Symbol(symbol), ty)?
            } else {
                self.settled_root(ty)?
            };
            self.report_unresolved_output_type(Origin::Symbol(symbol), ty, reported)?;
            resolved.push((symbol, ty));
        }

        Ok(resolved)
    }

    /// Resolve one module's alias definition values.
    ///
    /// Alias values write their reduced answer like alias symbol types.
    fn resolved_definition_values(
        &mut self,
        module: ModuleId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
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
            let value = self.resolved_output_type(Origin::Symbol(symbol), value)?;
            self.report_unresolved_output_type(Origin::Symbol(symbol), value, reported)?;
            resolved.push((symbol, value));
        }

        Ok(resolved)
    }

    /// Resolve one module's literal symbol values.
    fn static_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>> {
        let static_values = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();
        let mut literals = Vec::new();
        for (symbol, value) in static_values {
            let value = self.settled_root(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, *literal));
            }
        }

        Ok(literals)
    }

    /// Drain decided node meanings into output resolutions.
    fn drain_decisions(&mut self, module: ModuleId) {
        let decisions = self.solver.decisions.take_module(module);
        let resolutions = &mut self.module_mut(module).resolutions;
        for (node, decision) in decisions {
            match decision {
                Decision::Name(resolution) => {
                    resolutions.set_name_resolution(node, resolution);
                }
                Decision::Instantiation(resolution) => {
                    resolutions.set_instantiation_resolution(node, resolution);
                }
                Decision::Receiver(resolution) => {
                    resolutions.set_receiver_resolution(node, resolution);
                }
                Decision::Member(resolution) => {
                    resolutions.set_member_resolution(node, resolution);
                }
                Decision::Call(resolution) => {
                    resolutions.set_call_resolution(node, resolution);
                }
                Decision::ReadWrite(resolution) => {
                    resolutions.set_read_write_resolution(node, resolution);
                }
                Decision::Guard(resolution) => {
                    resolutions.set_guard_resolution(node, resolution);
                }
                Decision::Construct(resolution) => {
                    resolutions.set_construct_resolution(node, resolution);
                }
                Decision::Pattern(resolution) => {
                    resolutions.set_pattern_resolution(node, resolution);
                }
                Decision::AssignPattern(resolution) => {
                    resolutions.set_assign_pattern_resolution(node, resolution);
                }
                // rejections already carry their diagnostics
                Decision::Rejected => {}
            }
        }
    }
}
