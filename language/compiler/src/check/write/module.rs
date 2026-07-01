use destack_artifact::DiagnosticAnchor;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{Answer, CheckState, Decision, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::check) fn write_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let mut reported = IndexSet::new();
        let sealed_variables = self.sealed_variable_types(module)?;
        let node_types = self.resolved_node_types(module, &mut reported)?;
        let symbol_types = self.resolved_symbol_types(module, &mut reported)?;
        let reduced_types =
            self.resolved_reduced_types(module, &node_types, &symbol_types, &mut reported)?;
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

        // record inferred types and checked reduced types
        for (node, ty) in node_types {
            state.types.set_node_type(node, ty);
        }
        for (symbol, ty) in symbol_types {
            state.types.set_symbol_type(symbol, ty);
        }
        for (source, target) in reduced_types {
            state.types.set_type_reduction(source, target);
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

        // write closure capture frames and bindings
        self.write_captures(module, &mut reported)?;

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

    /// Resolve one module's recorded node types.
    fn resolved_node_types(
        &mut self,
        module: ModuleId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>> {
        let node_types = self
            .node_types
            .iter()
            .map(|(node, ty)| (*node, *ty))
            .filter_map(|(node, ty)| (node.module_id == module).then_some((node, ty)))
            .collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(node_types.len());
        for (node, ty) in node_types {
            let ty = self.resolved_node_type(ty)?;
            self.report_unresolved_output_type(Origin::Node(node), ty, reported)?;
            resolved.push((node, ty));
        }

        Ok(resolved)
    }

    /// Resolve one written node type to its settled type.
    fn resolved_node_type(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        self.settled_root(ty)
    }

    /// Resolve one written reduced type.
    fn resolved_reduced_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::GlobalTypeId)>> {
        let ty = self.settled_root(ty)?;
        if !self.type_variables(ty)?.is_empty() {
            return Ok(None);
        }

        match self.reduce_type(origin, ty)? {
            Answer::Ready(reduced) if reduced == ty => Ok(None),
            Answer::Ready(reduced) => {
                self.report_unresolved_output_type(origin, reduced, reported)?;

                Ok(Some((ty, reduced)))
            }
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
            let ty = self.settled_root(ty)?;
            self.report_unresolved_output_type(Origin::Symbol(symbol), ty, reported)?;
            resolved.push((symbol, ty));
        }

        Ok(resolved)
    }

    /// Resolve one module's checked reduced types.
    fn resolved_reduced_types(
        &mut self,
        module: ModuleId,
        node_types: &[(dir::GlobalNodeIdAny, dir::GlobalTypeId)],
        symbol_types: &[(dir::GlobalSymbolId, dir::GlobalTypeId)],
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Vec<(dir::GlobalTypeId, dir::GlobalTypeId)>> {
        let mut sources = Vec::new();
        sources.extend(
            node_types
                .iter()
                .map(|(node, ty)| (Origin::Node(*node), *ty)),
        );
        sources.extend(
            symbol_types
                .iter()
                .map(|(symbol, ty)| (Origin::Symbol(*symbol), *ty)),
        );
        sources.extend(
            self.module(module)
                .definitions
                .iter_definitions()
                .filter_map(|(symbol, definition)| match definition {
                    dir::Definition::TypeAlias(definition) => {
                        Some((Origin::Symbol(symbol), definition.value))
                    }
                    _ => None,
                }),
        );

        let mut seen = IndexSet::new();
        let mut resolved = Vec::new();
        for (origin, ty) in sources {
            if !seen.insert(ty) {
                continue;
            }

            if let Some(reduction) = self.resolved_reduced_type(origin, ty, reported)? {
                resolved.push(reduction);
            }
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
        let decisions = self.decisions.take_module(module);
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
                Decision::Place(resolution) => {
                    resolutions.set_place_resolution(node, resolution);
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
