use std::sync::Arc;

use destack_artifact::DirMaterialized;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::materialize::InstanceWorklist;
use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Run the materialize pass: evaluate open computations and close instances.
    pub(in crate::sema) fn run_materialize(&mut self) -> CompilerResult<()> {
        // load the external modules this module reads
        self.import_external_modules()?;

        // materialize the module's own entries, then the instances they reach
        let mut worklist = InstanceWorklist::default();
        self.materialize_module_entries(&mut worklist)?;
        self.materialize_field_types()?;
        self.materialize_member_bodies(&mut worklist)?;
        self.materialize_instances(&mut worklist)
    }

    /// Close each field's type at its materialized symbol type, undefined included for optional fields.
    fn materialize_field_types(&mut self) -> CompilerResult<()> {
        let definitions: Vec<dir::GlobalSymbolId> = self
            .module
            .iter_definitions()
            .map(|(symbol, _)| symbol)
            .collect();
        for owner in definitions {
            let Some(definition) = self.module.definition(owner) else {
                return Err(CompilerError::Internal {
                    message: "a listed definition without its record".to_string(),
                });
            };
            let fields: Vec<(dir::GlobalSymbolId, bool)> = definition
                .members()
                .iter()
                .filter_map(|member| match member {
                    dir::DefinitionMember::Field(field) => Some((field.symbol, field.is_optional)),
                    _ => None,
                })
                .collect();
            for (symbol, is_optional) in fields {
                let Some(declared) = self.symbol_type_maybe(symbol) else {
                    continue;
                };
                let ty = match is_optional {
                    true => {
                        let undefined = self.intern_type(dir::Type::Undefined)?;
                        self.normalized_union_type([declared, undefined])?
                    }
                    false => declared,
                };
                if let Some(definition) = self.definition_mut(owner)
                    && let Some(field) =
                        definition
                            .members_mut()
                            .iter_mut()
                            .find_map(|member| match member {
                                dir::DefinitionMember::Field(field) if field.symbol == symbol => {
                                    Some(field)
                                }
                                _ => None,
                            })
                {
                    field.ty = ty;
                }
            }
        }

        Ok(())
    }

    /// Convert materialized state into one materialized DIR module.
    pub(in crate::sema) fn into_materialized(mut self) -> CompilerResult<DirMaterialized> {
        // write this pass's results back into the module state
        let module = self.module_id;
        self.commit_instance_conformances(module)?;
        self.materialize_union_members(module)?;

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
            decisions: Arc::new(module_state.decisions_tail),
            coercions: Arc::new(module_state.coercions_tail),
            roots,
        })
    }

    /// Record the canonical flat member list for every union the module's types resolve to.
    fn materialize_union_members(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect the ids before writing member lists into the segment
        let state = self.module(module);
        let ids: Vec<dir::LocalTypeId> = state
            .types
            .iter_type_ids()
            .chain(state.types_tail.iter_type_ids())
            .collect();

        for local in ids {
            let ty = local.into_global(module);

            // keep the member list one earlier pass recorded
            let state = self.module(module);
            let recorded = state
                .types_tail
                .get_union_members(ty)
                .or_else(|| state.types.get_union_members(ty));
            if recorded.is_some() {
                continue;
            }

            // leave open inference and error types to the passes that own them
            let flags = self.type_flags(ty)?;
            if flags.has_variable() || flags.has_infer() || flags.has_error() {
                continue;
            }

            // record union heads only
            let dir::Type::Union(union) = self.ty(ty)? else {
                continue;
            };

            // load the modules the member types live in
            let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
            for element in elements {
                if !self.is_own_module(element.module_id) {
                    self.import_external_module(element.module_id)?;
                }
            }

            // anchor the sweep at the module node
            let anchor = self.module(module).bound.module_node;
            let origin = Origin::Node(anchor.into_global(module), None);
            let Some(members) = self.canonical_union_members(origin, ty)? else {
                continue;
            };

            // commit the member list into this pass's segment
            let list = self.intern_type_ids(&members)?;
            self.module_mut(module)
                .types_tail
                .set_union_members(ty, list);
        }

        Ok(())
    }
}
