use destack_artifact::DiagnosticAnchor;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{Answer, CheckError, CheckState, Decision, Origin};
use crate::{CompilerError, CompilerResult};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Finish one module's solved state into output DIR tables.
    pub(in crate::check) fn finish_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<CheckModuleOutput> {
        let mut reported = IndexSet::new();
        let patches = self.finish_variable_patches(module)?;
        let node_types = self.finish_node_types(module, &mut reported)?;
        let symbol_types = self.finish_symbol_types(module, &mut reported)?;
        let definition_values = self.finish_definition_values(module, &mut reported)?;
        let symbol_literals = self.finish_symbol_literals(module)?;

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
        for (local, patched) in patches {
            output.types.update_type(local, patched);
        }

        // alias definition values finish evaluated like their symbol types
        for (symbol, value) in definition_values {
            if let Some(dir::Definition::TypeAlias(definition)) =
                output.definitions.definition_mut(symbol)
            {
                definition.value = value;
            }
        }

        // record inferred node and symbol types
        for (node, ty) in node_types {
            output.types.set_node_type(node, ty);
        }
        for (symbol, ty) in symbol_types {
            output.types.set_symbol_type(symbol, ty);
        }

        // record implicit coercions beside their value nodes
        for (node, coercion) in self.module_coercions(module) {
            output.coercions.bind_coercion(node, coercion);
        }

        // symbol values materialize as final statics
        for (symbol, literal) in symbol_literals {
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
        for (local, variable) in variables {
            let representative = self.variables.representative(variable)?;
            let solution = self.variables.solution(representative)?;
            let patched = match solution {
                Some(solution) => {
                    let solved = self.settled_root(solution)?;

                    self.ty(solved)?.clone()
                }
                None => dir::Type::Error,
            };
            patches.push((local, patched));
        }

        Ok(patches)
    }

    /// Return whether one origin already has a primary rejection diagnostic.
    fn origin_has_rejected_decision(&self, origin: Origin) -> CompilerResult<bool> {
        match origin {
            Origin::Node(node) => Ok(self.node_has_rejected_decision(node)),
            Origin::Symbol(symbol) => self.symbol_has_rejected_initializer(symbol),
            Origin::Type(_) => Ok(false),
        }
    }

    /// Return whether one node or enclosing expression already rejected.
    fn node_has_rejected_decision(&self, node: dir::GlobalNodeIdAny) -> bool {
        let module = self.module(node.module_id);
        let view = module.view();
        let mut current = node.local_id;
        loop {
            let node = current.into_global(node.module_id);
            if matches!(self.decisions.get(node), Some(Decision::Rejected)) {
                return true;
            }

            let Some(parent) = view.tree().get_parent(current.id) else {
                return false;
            };
            current = parent;
        }
    }

    /// Return whether one symbol's initializer already rejected.
    fn symbol_has_rejected_initializer(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let module = self.module(symbol.module_id);
        let source = module.symbol_declaration_node(symbol.local_id)?;
        let view = module.view();
        let Some(parent) = view.tree().get_parent(source.id) else {
            return Ok(false);
        };
        if parent.ty != dir::NodeType::Declarator {
            return Ok(false);
        }

        let declarator = view.get(parent.into_typed::<dir::Declarator>());
        let Some(value) = declarator.value else {
            return Ok(false);
        };

        Ok(self.node_has_rejected_decision(value.into_global_any(symbol.module_id)))
    }

    /// Resolve one module's recorded node types.
    fn finish_node_types(
        &mut self,
        module: ModuleId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>> {
        let node_types = self.module(module).types.node_types().collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(node_types.len());
        for (node, ty) in node_types {
            let ty = self.settled_root(ty)?;
            self.report_unresolved_output_type(Origin::Node(node), ty, reported)?;
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
        let ty = self.settled_root(ty)?;
        match self.evaluate_type(origin, ty)? {
            Answer::Ready(evaluated) => Ok(evaluated),
            Answer::Pending(blockers) => {
                let (_, anchor) = self.origin_diagnostic_anchor(origin)?;

                Err(CompilerError::Internal {
                    message: format!(
                        "finished output type for {origin:?} at {anchor:?} is still pending: {blockers:?}"
                    ),
                })
            }
        }
    }

    /// Resolve one module's recorded symbol types.
    fn finish_symbol_types(
        &mut self,
        module: ModuleId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let symbol_types = self.module(module).types.symbol_types().collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(symbol_types.len());
        for (symbol, ty) in symbol_types {
            // alias values finish their evaluated answer, every other
            // symbol keeps its written spelling for lazy use sites
            let ty = if self.symbol_kind(symbol) == dir::SymbolKind::TypeAlias {
                self.finish_type(Origin::Symbol(symbol), ty)?
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
    /// Alias values finish their evaluated answer like alias symbol types.
    fn finish_definition_values(
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
            let value = self.finish_type(Origin::Symbol(symbol), value)?;
            self.report_unresolved_output_type(Origin::Symbol(symbol), value, reported)?;
            resolved.push((symbol, value));
        }

        Ok(resolved)
    }

    /// Report one final output type that still references inference variables.
    fn report_unresolved_output_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<()> {
        if self.type_variables(ty)?.is_empty() || self.origin_has_rejected_decision(origin)? {
            return Ok(());
        }

        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        if self.modules.contains_key(&module) && reported.insert((module, anchor.clone())) {
            let error = CheckError::MissingTypeAnnotation { anchor, module };
            self.module_mut(module).diagnostics.push(error.into());
        }

        Ok(())
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
            let value = self.settled_root(value)?;
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
                Decision::Instantiation(resolution) => {
                    output
                        .resolutions
                        .set_instantiation_resolution(node, resolution.clone());
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
                Decision::Guard(resolution) => {
                    output
                        .resolutions
                        .set_guard_resolution(node, resolution.clone());
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
