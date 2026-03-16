use crate::analyze::common::TypeContext;
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, BuildKey, BuildRequirementCollector, BuildRequirementError,
    Compiler,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleDir, ProfileId};
use std::sync::Arc;

impl Compiler {
    /// Ensure interface DIR exists for a module.
    pub fn require_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        // ensure the component graph exists before selecting an anchor
        self.require_resolved_dependency_closure([module], profile)?;

        // avoid self dependency when already analyzing this module interface
        if self.current_build_key()
            == Some(BuildKey::Artifact(
                destack_workspace::ArtifactKey::DirInterface { module, profile },
            ))
        {
            return Ok(());
        }

        // avoid same-component self cycles only from the canonical anchor task
        if let Some(BuildKey::Artifact(destack_workspace::ArtifactKey::DirInterface {
            module: current_module,
            profile: current_profile,
        })) = self.current_build_key()
            && current_profile == profile
        {
            let current_anchor = self.interface_component_anchor_module_id(current_module, profile);
            if current_anchor == current_module {
                let graph_key = destack_workspace::ModuleGraphKey::new(profile);
                let shares_component = self
                    .program
                    .index
                    .module_graphs
                    .get(&graph_key)
                    .map(|graph| {
                        let index = self.interface_component_graph_index(profile, &graph);
                        let current_component_id = index.component_id_for_module(current_module);
                        let target_component_id = index.component_id_for_module(module);
                        current_component_id.is_some()
                            && current_component_id == target_component_id
                    })
                    .unwrap_or(current_module == module);
                if shares_component {
                    return Ok(());
                }
            }
        }

        let anchor_module_id = self.interface_component_anchor_module_id(module, profile);
        self.require_build_key(BuildKey::Artifact(
            destack_workspace::ArtifactKey::DirInterface {
                module: anchor_module_id,
                profile,
            },
        ))
    }

    /// Phase 2: Build interface summaries.
    pub(crate) fn analyze_module_interface_inner(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<Arc<ModuleDir>> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_INTERFACE);

        // ensure local declarations are ready
        self.require_dir_declared(module_id, profile)?;

        // load module state and dir ctx
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            let dir_data = self
                .require_artifact_dir(destack_workspace::ArtifactKey::dir_declared(
                    module_id, profile,
                ))
                .map_err(AnalyzeError::from)?;
            return Ok(dir_data);
        }

        // read the module dir ctx for analysis
        let mut dir = self
            .require_artifact_dir(destack_workspace::ArtifactKey::dir_declared(
                module_id, profile,
            ))
            .map_err(AnalyzeError::from)?;
        {
            let dir = Arc::make_mut(&mut dir);
            let ModuleDir {
                tree,
                symbols,
                types,
                roots,
                anchor_node,
                namespace_symbol,
                namespace_exports,
                module_bindings,
                module_binding_exports,
                exported_symbols,
                ..
            } = dir;
            let types = Arc::make_mut(types);
            let mut collector = BuildRequirementCollector::new();

            if self.is_code_module(module_id) {
                let options = self.analyze_context_options_for_module(module_id);
                let mut ctx = TypeContext::new(
                    &module,
                    profile,
                    &options,
                    tree.as_ref(),
                    symbols.as_ref(),
                    types,
                );

                {
                    let _timing = self.timing_scope(tags::ANALYZE_INTERFACE_VALUES);

                    // declare exported value types with local-only inference
                    self.collect(
                        &mut collector,
                        self.infer_interface_value_types(
                            &mut ctx.reborrow(),
                            exported_symbols.as_ref(),
                            false,
                        ),
                    );

                    // declare exported value types for module bindings
                    for exports in module_binding_exports
                        .values()
                        .map(|binding| &binding.exports)
                    {
                        self.collect(
                            &mut collector,
                            self.infer_interface_value_types(&mut ctx.reborrow(), exports, false),
                        );
                    }
                }

                {
                    let _timing = self.timing_scope(tags::ANALYZE_INTERFACE_NAMESPACE);
                    let module_source_id = roots
                        .first()
                        .copied()
                        .map(destack_dir::LocalNodeId::into_any)
                        .unwrap_or(*anchor_node);

                    // declare the module namespace value type from exports
                    self.collect(
                        &mut collector,
                        self.collect_module_namespace_value_type(
                            &mut ctx.reborrow(),
                            exported_symbols.as_ref(),
                            module_source_id,
                            *namespace_symbol,
                            namespace_exports.as_ref(),
                            module_bindings.as_ref(),
                            module_binding_exports.as_ref(),
                        ),
                    );
                }
            }

            // yield on any yields
            if let Some(requirement) = collector.try_into_requirement() {
                return Err(AnalyzeError::Yield { requirement });
            }
        }
        Ok(dir)
    }
}
