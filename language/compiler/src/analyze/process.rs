use crate::{AnalyzeError, AnalyzeResult, BuildDependency, BuildKey, Compiler};
use destack_dir::CaptureTable;
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, DirAnalyzed, DirDeclared, ProfileId};

impl Compiler {
    /// Build declared DIR for one module.
    pub fn process_dir_declared(&self, module: ModuleId, profile: ProfileId) -> AnalyzeResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<AnalyzeError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        // transient declared builder
        let resolved = self
            .require_artifact_dir_resolved(module, profile)
            .map_err(AnalyzeError::from)?;
        let mut symbols = resolved.symbols.as_ref().clone();
        let mut types = resolved.types.as_ref().clone();
        let mut captures = CaptureTable::new();
        self.analyze_module_declare(
            resolved.as_ref(),
            &mut symbols,
            &mut types,
            &mut captures,
            module,
            profile,
            module_version,
            profile_version,
        )?;

        self.program.artifacts.publish(
            ArtifactKey::DirDeclared { module, profile },
            DirDeclared::from_resolved_with(resolved.as_ref(), symbols, types, captures),
        );

        Ok(())
    }

    /// Build interface DIR for one module.
    pub fn process_dir_interface(&self, module: ModuleId, profile: ProfileId) -> AnalyzeResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<AnalyzeError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        let entries =
            self.analyze_module_interface(module, profile, module_version, profile_version)?;

        for (module_id, profile_id, dir) in entries {
            let artifact_key = ArtifactKey::DirInterface {
                module: module_id,
                profile: profile_id,
            };
            let build_key = BuildKey::artifact(artifact_key.clone());
            let dependency = self.build_dependency_for_key(&build_key);

            self.program.artifacts.publish(
                ArtifactKey::DirInterface {
                    module: module_id,
                    profile: profile_id,
                },
                dir,
            );
            if let BuildDependency::Artifact(dependency) = dependency {
                self.program
                    .artifacts
                    .set_dependency(artifact_key, dependency);
            }
        }

        Ok(())
    }

    /// Build analyzed DIR for one module.
    pub fn process_dir_analyzed(&self, module: ModuleId, profile: ProfileId) -> AnalyzeResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<AnalyzeError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        // transient analyzed builder
        let interface = self
            .require_artifact_dir_interface(module, profile)
            .map_err(AnalyzeError::from)?;
        let declared = self
            .require_artifact_dir_declared(module, profile)
            .map_err(AnalyzeError::from)?;
        let tree = interface.tree.clone();
        let symbols = interface.symbols.clone();
        let roots = interface.roots.clone();
        let anchor_node = interface.anchor_node;
        let default_symbol = declared.default_symbol;
        let mut types = interface.types.as_ref().clone();
        let mut captures = declared.captures.as_ref().clone();
        let mut infer_table = self.analyze_module_infer(
            tree.as_ref(),
            symbols.as_ref(),
            roots.as_ref(),
            &mut types,
            default_symbol,
            anchor_node,
            module,
            profile,
            module_version,
            profile_version,
        )?;
        self.analyze_module_solve(
            tree.as_ref(),
            symbols.as_ref(),
            &mut types,
            infer_table.as_mut(),
            module,
            profile,
            module_version,
            profile_version,
        )?;
        self.analyze_module_commit(
            tree.as_ref(),
            symbols.as_ref(),
            &mut types,
            infer_table.as_mut(),
            module,
            profile,
            module_version,
            profile_version,
        )?;
        self.analyze_module_capture(
            tree.as_ref(),
            symbols.as_ref(),
            &mut captures,
            module,
            profile,
            module_version,
            profile_version,
        )?;
        self.analyze_module_validate(
            tree.as_ref(),
            symbols.as_ref(),
            &mut types,
            anchor_node,
            module,
            profile,
            module_version,
            profile_version,
        )?;

        // publish the analyzed artifact from interface inputs plus local semantic tables
        let payload = DirAnalyzed::from_interface_and_declared_with(
            interface.as_ref(),
            declared.as_ref(),
            types,
            captures,
        );

        let cache_handle = self.cache_handle_for_module(
            module,
            Some(profile),
            None,
            destack_source::CacheKind::DirAnalyzed,
        );
        if let Some(cache) = cache_handle.as_ref() {
            if let Err(error) = cache.write_dir_analyzed(payload.clone()) {
                tracing::debug!(?module, ?profile, ?error, "analyze.module.cache.write");
            }
        }

        if self.is_code_module(module) {
            self.stats.record_analyze();
        }

        self.program
            .artifacts
            .publish(ArtifactKey::DirAnalyzed { module, profile }, payload);

        Ok(())
    }
}
