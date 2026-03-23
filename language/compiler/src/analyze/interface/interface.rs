use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, ArtifactRequirementCollector, ArtifactRequirementError, Compiler,
};
use destack_artifact::{ArtifactKey, DirInterface};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;
use std::sync::Arc;

impl Compiler {
    /// Ensure interface DIR exists for a module.
    pub fn require_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), ArtifactRequirementError> {
        // ensure the component graph exists before selecting an anchor
        self.require_resolved_dependency_closure([module], profile)?;

        // avoid self dependency when already analyzing this module interface
        if self.current_artifact_key() == Some(ArtifactKey::dir_interface(module, profile)) {
            return Ok(());
        }

        // avoid same-component self cycles only from the canonical anchor task
        if let Some(ArtifactKey::DirInterface {
            module: current_module,
            profile: current_profile,
        }) = self.current_artifact_key()
            && current_profile == profile
        {
            let current_anchor = self.interface_component_anchor_module_id(current_module, profile);
            if current_anchor == current_module {
                let shares_component = self
                    .artifacts
                    .module_graph(profile)
                    .and_then(|_| self.interface_component_graph_index(profile))
                    .map(|index| {
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
        self.require_artifact(ArtifactKey::dir_interface(anchor_module_id, profile))
    }

    /// Phase 2: Build interface summaries.
    pub(crate) fn analyze_module_interface_inner(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<Arc<DirInterface>> {
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
            let resolved = self
                .require_artifact_dir_resolved(module_id, profile)
                .map_err(AnalyzeError::from)?;
            let dir = self
                .require_artifact_dir_declared(module_id, profile)
                .map_err(AnalyzeError::from)?;
            return Ok(Arc::new(DirInterface::from_resolved_and_declared(
                resolved.as_ref(),
                dir.as_ref(),
            )));
        }

        // read the module dir ctx for analysis
        let resolved = self
            .require_artifact_dir_resolved(module_id, profile)
            .map_err(AnalyzeError::from)?;
        let dir = self
            .require_artifact_dir_declared(module_id, profile)
            .map_err(AnalyzeError::from)?;
        let base = self
            .require_artifact_dir_base(module_id)
            .map_err(AnalyzeError::from)?;
        let tree = dir.tree.clone();
        let symbols = dir.symbols.clone();
        let roots = dir.roots.clone();
        let anchor_node = dir.anchor_node;
        let namespace_symbol = dir.namespace_symbol;
        let namespace_exports = resolved.namespace_exports.clone();
        let module_bindings = base.module_bindings.clone();
        let module_binding_exports = resolved.module_binding_exports.clone();
        let exported_symbols = resolved.exported_symbols.clone();
        let mut types = dir.types.as_ref().clone();
        {
            let mut collector = ArtifactRequirementCollector::new();

            if self.is_code_module(module_id) {
                let options = self.analyze_context_options_for_module(module_id);
                let mut ctx = TypeContext::new(
                    &module,
                    profile,
                    &options,
                    tree.as_ref(),
                    symbols.as_ref(),
                    &mut types,
                    AnalyzeIndex::default(),
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
                        .unwrap_or(anchor_node);

                    // declare the module namespace value type from exports
                    self.collect(
                        &mut collector,
                        self.collect_module_namespace_value_type(
                            &mut ctx.reborrow(),
                            exported_symbols.as_ref(),
                            module_source_id,
                            namespace_symbol,
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

        Ok(Arc::new(
            DirInterface::from_resolved_and_declared_with_types(
                resolved.as_ref(),
                dir.as_ref(),
                types,
            ),
        ))
    }
}
