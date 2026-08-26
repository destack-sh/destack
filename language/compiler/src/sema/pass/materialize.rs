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
        self.materialize_member_bodies(&mut worklist)?;
        self.materialize_instances(&mut worklist)
    }

    /// Evaluate each committed definition's types to their closed forms.
    fn materialize_definitions(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the committed rows once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalSymbolId, dir::Definition)> = self
            .module
            .iter_definitions()
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect();

        // evaluate every embedded type and keep the definitions evaluation moves
        for (symbol, definition) in committed {
            let Some(source) = self.module.definition_source_maybe(symbol) else {
                continue;
            };

            let origin = Origin::Node(source, None);
            let mut resolved = definition.clone();
            dir::TypeFold::map_types(&mut resolved, &mut |ty| -> CompilerResult<_> {
                // keep open template types written; instances materialize them
                let flags = self.type_flags(ty)?;
                if flags.has_parameter() || flags.has_this() || flags.has_variable() {
                    return Ok(ty);
                }

                // evaluate only computation results, keeping written aliases as written
                let resolved = match self.has_reachable_computation(ty)? {
                    true => self.evaluate_type(origin, ty)?,
                    false => ty,
                };

                // intern the concrete applications the evaluated type reaches
                self.intern_applications(resolved, source, 0, worklist)?;

                Ok(resolved)
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

    /// Materialize each committed symbol type.
    fn materialize_symbol_types(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the committed rows once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)> =
            self.module.types.symbol_types().collect();

        // materialize every closed symbol type, overriding the ones evaluation moves
        for (symbol, ty) in committed {
            let flags = self.type_flags(ty)?;
            if flags.has_parameter() || flags.has_this() || flags.has_variable() {
                continue;
            }

            // skip synthesized symbols without a declaration
            let declaration = self
                .binding_table(symbol.module_id)
                .get_symbol(symbol.local_id)
                .declaration;
            let Some(source) = declaration else {
                continue;
            };

            // evaluate only computation results, keeping written aliases as written
            let mut resolved = ty;
            if self.has_reachable_computation(ty)? {
                let origin = Origin::Node(source, None);
                resolved = self.evaluate_type(origin, ty)?;
                if resolved != ty {
                    self.module.types_tail.set_symbol_type(symbol, resolved);
                }
            }

            // intern the concrete applications the materialized type reaches
            self.intern_applications(resolved, source, 0, worklist)?;
        }

        Ok(())
    }

    /// Materialize each committed node type.
    fn materialize_node_types(&mut self, worklist: &mut InstanceWorklist) -> CompilerResult<()> {
        // collect the committed rows once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)> =
            self.module.types.node_types().collect();

        // materialize every closed node type, overriding the ones evaluation moves
        for (node, ty) in committed {
            let flags = self.type_flags(ty)?;
            if flags.has_parameter() || flags.has_this() || flags.has_variable() {
                continue;
            }

            // evaluate only computation results, keeping written aliases as written
            let mut resolved = ty;
            if self.has_reachable_computation(ty)? {
                let origin = Origin::Node(node, None);
                resolved = self.evaluate_type(origin, ty)?;
                if resolved != ty {
                    self.module.types_tail.set_node_type(node, resolved);
                }
            }

            // intern the concrete applications the materialized type reaches
            self.intern_applications(resolved, node, 0, worklist)?;
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

        let parameters = template.parameters.clone();

        // any parameter beyond the memory kinds makes the symbol generic,
        //  since semantic identity is region and space free
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
