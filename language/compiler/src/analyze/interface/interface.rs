use crate::analyze::DirReadBoundary;
use crate::analyze::common::TypeContext;
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, BuildKey, BuildRequirementCollector, BuildRequirementError,
    Compiler,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ArtifactKey, ModuleDir, ProfileId};

impl Compiler {
    /// Ensure interface DIR exists for a module.
    pub fn require_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        // current local build frame already satisfies interface reads
        if self
            .current_active_dir_frame(module, profile, DirReadBoundary::Interface)
            .is_some()
        {
            return Ok(());
        }

        // ensure the component graph exists before selecting an anchor
        self.require_interface_forward_closure(module, profile)?;

        // avoid self dependency when already analyzing this module interface
        if self.current_build_key()
            == Some(BuildKey::Artifact(ArtifactKey::DirInterface {
                module,
                profile,
            }))
        {
            return Ok(());
        }

        // avoid same-component self cycles only from the canonical anchor task
        if let Some(BuildKey::Artifact(ArtifactKey::DirInterface {
            module: current_module,
            profile: current_profile,
        })) = self.current_build_key()
            && current_profile == profile
        {
            let current_anchor = self.interface_component_anchor_module_id(current_module, profile);
            if current_anchor == current_module
                && self.interface_modules_share_component(profile, current_module, module)
            {
                return Ok(());
            }
        }

        let anchor_module_id = self.interface_component_anchor_module_id(module, profile);
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirInterface {
            module: anchor_module_id,
            profile,
        }))
    }

    /// Phase 2: Build interface summaries.
    pub(crate) fn analyze_module_interface_inner(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<ModuleDir> {
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
        let module = module.read();

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            let dir_data = self
                .require_artifact_dir_for_boundary(module_id, profile, DirReadBoundary::Declared)
                .map_err(AnalyzeError::from)?;
            let (_, dir) = self.with_shared_transient_artifact_dir(
                module_id,
                profile,
                DirReadBoundary::Interface,
                dir_data,
                |_dir| Ok::<(), AnalyzeError>(()),
            )?;
            return Ok(dir);
        }

        // read the module dir ctx for analysis
        let dir_data = self
            .require_artifact_dir_for_boundary(module_id, profile, DirReadBoundary::Declared)
            .map_err(AnalyzeError::from)?;
        let (_, dir) = self.with_shared_transient_artifact_dir(
            module_id,
            profile,
            DirReadBoundary::Interface,
            dir_data,
            |dir| -> AnalyzeResult<()> {
                let tree = dir.tree.read();
                let mut types = dir.types.write();
                let symbols = dir.symbols.read();
                let mut collector = BuildRequirementCollector::new();

                if !self.is_code_module(module_id) {
                    return Ok(());
                }

                let exported_symbols = dir.exported_symbols.read();
                let binding_exports = dir.module_binding_exports.read();
                let options = self.analyze_context_options_for_module(module_id);
                let mut ctx = TypeContext::with_dir(
                    &module,
                    profile,
                    &options,
                    dir.as_ref(),
                    &tree,
                    &symbols,
                    &mut types,
                );

                {
                    let _timing = self.timing_scope(tags::ANALYZE_INTERFACE_VALUES);

                    // declare exported value types with local-only inference
                    self.collect(
                        &mut collector,
                        self.infer_interface_value_types(
                            &mut ctx.reborrow(),
                            &exported_symbols,
                            false,
                        ),
                    );

                    // declare exported value types for module bindings
                    for binding in binding_exports.values() {
                        self.collect(
                            &mut collector,
                            self.infer_interface_value_types(
                                &mut ctx.reborrow(),
                                &binding.exports,
                                false,
                            ),
                        );
                    }
                }

                {
                    let _timing = self.timing_scope(tags::ANALYZE_INTERFACE_NAMESPACE);

                    // declare the module namespace value type from exports
                    self.collect(
                        &mut collector,
                        self.collect_module_namespace_value_type(
                            &mut ctx.reborrow(),
                            &exported_symbols,
                        ),
                    );
                }

                // yield on any yields
                if let Some(requirement) = collector.try_into_requirement() {
                    return Err(AnalyzeError::Yield { requirement });
                }

                Ok(())
            },
        )?;
        drop(module);
        Ok(dir)
    }
}
