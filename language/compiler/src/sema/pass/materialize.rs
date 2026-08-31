use std::sync::Arc;

use destack_artifact::DirMaterialized;
use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::materialize::InstanceWorklist;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Run the materialize pass: evaluate open computations and close instances.
    pub(in crate::sema) fn run_materialize(&mut self) -> CompilerResult<()> {
        // load the external modules this module reads
        self.import_external_modules()?;

        // materialize the module's own types, then the instances they reach
        let mut worklist = InstanceWorklist::default();
        self.materialize_definitions(&mut worklist)?;
        self.materialize_symbol_types(&mut worklist)?;
        self.materialize_node_types(&mut worklist)?;
        self.materialize_payloads(&mut worklist)?;
        self.materialize_member_bodies(&mut worklist)?;
        self.materialize_instances(&mut worklist)
    }

    /// Materialize one committed type.
    ///
    /// Solved variables substitute into the spelling.
    /// This-typed and unsolved types stay written for their instances.
    /// Template types stay written and intern their closed applications.
    /// Closed types evaluate the computations they reach and intern the result's applications.
    fn materialize_type(
        &mut self,
        anchor: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(anchor, None);

        // resolve solved variables into the committed spelling
        let mut ty = ty;
        let mut flags = self.type_flags(ty)?;
        if flags.has_variable() {
            ty = self.deeply_resolve(origin, ty)?;
            flags = self.type_flags(ty)?;
        }

        // close this-typed and unsolved types under their instances
        if flags.has_this() || flags.has_variable() {
            return Ok(ty);
        }

        // keep template types written, interning their closable applications
        if flags.has_parameter() {
            self.intern_applications(ty, anchor, 0, worklist)?;

            return Ok(ty);
        }

        // evaluate only computation results, keeping written aliases as written
        let resolved = match self.has_reachable_computation(ty)? {
            true => self.evaluate_type(origin, ty)?,
            false => ty,
        };
        self.intern_applications(resolved, anchor, 0, worklist)?;

        Ok(resolved)
    }

    /// Close the committed definitions, keeping the ones evaluation moves.
    fn materialize_definitions(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the committed definitions once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalSymbolId, dir::Definition)> = self
            .module
            .iter_definitions()
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect();

        for (symbol, definition) in committed {
            let Some(source) = self.module.definition_source_maybe(symbol) else {
                continue;
            };

            // close every type the definition embeds
            let mut resolved = definition.clone();
            dir::TypeFold::map_types(&mut resolved, &mut |ty| {
                self.materialize_type(source, ty, worklist)
            })?;

            // override only the definitions evaluation moves
            if resolved != definition {
                self.module
                    .definitions_tail
                    .insert_definition(symbol, source, resolved);
            }
        }

        Ok(())
    }

    /// Close the committed symbol types, keeping the ones evaluation moves.
    fn materialize_symbol_types(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the committed symbol types once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)> =
            self.module.types.symbol_types().collect();

        for (symbol, ty) in committed {
            // skip synthesized symbols without a declaration
            let declaration = self
                .binding_table(symbol.module_id)
                .get_symbol(symbol.local_id)
                .declaration;
            let Some(source) = declaration else {
                continue;
            };

            // override only the symbol types evaluation moves
            let resolved = self.materialize_type(source, ty, worklist)?;
            if resolved != ty {
                self.module.types_tail.set_symbol_type(symbol, resolved);
            }
        }

        Ok(())
    }

    /// Close the committed node types, keeping the ones evaluation moves.
    fn materialize_node_types(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the committed node types once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)> =
            self.module.types.node_types().collect();

        for (node, ty) in committed {
            // override only the node types evaluation moves
            let resolved = self.materialize_type(node, ty, worklist)?;
            if resolved != ty {
                self.module.types_tail.set_node_type(node, resolved);
            }
        }

        Ok(())
    }

    /// Close the committed decisions, place resolutions, and coercions.
    fn materialize_payloads(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        let Some(checked) = self.module.checked.clone() else {
            return Ok(());
        };

        // close the types each committed decision carries
        for (node, decision) in checked.decisions.decision_entries() {
            let mut resolved = decision.clone();
            dir::TypeFold::map_types(&mut resolved, &mut |ty| {
                self.materialize_type(node, ty, worklist)
            })?;
            if resolved != *decision {
                self.module.decisions.set_decision(node, resolved);
            }
        }

        // close the types each committed place resolution carries
        for (node, place) in checked.decisions.place_entries() {
            let mut resolved = *place;
            dir::TypeFold::map_types(&mut resolved, &mut |ty| {
                self.materialize_type(node, ty, worklist)
            })?;
            if resolved != *place {
                self.module.decisions.set_place_resolution(node, resolved);
            }
        }

        // close the types each committed coercion carries
        for (node, coercion) in checked.coercions.coercions() {
            let mut resolved = coercion.clone();
            dir::TypeFold::map_types(&mut resolved, &mut |ty| {
                self.materialize_type(node, ty, worklist)
            })?;
            if resolved != *coercion {
                self.module.coercions.bind_coercion(node, resolved);
            }
        }

        Ok(())
    }

    /// Materialize the `this`-typed payloads of non-generic member bodies.
    fn materialize_member_bodies(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the methods the module's own definitions declare
        let mut members = Vec::new();
        for (owner, definition) in self.module.iter_definitions() {
            for member in definition.members() {
                if let dir::DefinitionMember::Method(method) = member {
                    members.push((owner, method.symbol));
                }
            }
        }

        // materialize each body outside a generic; instances cover the rest
        for (owner, member) in members {
            if self.symbol_has_type_parameters(owner)? || self.symbol_has_type_parameters(member)? {
                continue;
            }

            self.materialize_member_body(owner, member, worklist)?;
        }

        Ok(())
    }

    /// Return whether one symbol's template declares type parameters.
    pub(in crate::sema) fn symbol_has_type_parameters(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(template_id) = self.symbol_template(symbol)? else {
            return Ok(false);
        };

        let Some(template) = self.generic_template(template_id) else {
            return Ok(false);
        };

        // accept any parameter beyond the memory kinds
        let parameters = template.parameters.clone();
        for parameter in parameters {
            let parameter = parameter.into_global(template_id.module_id);
            let is_memory = self
                .generic_parameter(parameter)
                .is_some_and(|binding| binding.memory_parameter().is_some());
            if !is_memory {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Convert materialized state into one materialized DIR module.
    pub(in crate::sema) fn into_materialized(mut self) -> CompilerResult<DirMaterialized> {
        // write this pass's results back into the module state
        self.write_back()?;
        let module = self.module_id;
        self.commit_instance_conformances(module)?;

        // take the tail segments this pass wrote
        let parsed = Arc::clone(&self.module.parsed);
        let roots = self.module.expanded.roots.clone();
        let module_state = self.module;
        let types = module_state.types_tail.finish();

        Ok(DirMaterialized {
            patch: dir::Patch::new(&parsed.tree, "materialize"),
            types: Arc::new(types),
            generics: Arc::new(module_state.generics_tail),
            definitions: Arc::new(module_state.definitions_tail),
            decisions: Arc::new(module_state.decisions),
            coercions: Arc::new(module_state.coercions),
            roots,
        })
    }
}
