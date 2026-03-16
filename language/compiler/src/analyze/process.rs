use crate::{AnalyzeError, AnalyzeResult, BuildDependency, BuildKey, Compiler};
use destack_source::ModuleId;
use destack_workspace::ProfileId;
use std::sync::Arc;

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
        let mut dir = self
            .require_artifact_dir_resolved(module, profile)
            .map_err(AnalyzeError::from)?;
        {
            let dir = Arc::make_mut(&mut dir);
            self.analyze_module_declare(dir, module, profile, module_version, profile_version)?;
        }

        self.program
            .artifacts
            .set_dir_declared(module, profile, dir);

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
            let artifact_key = destack_workspace::ArtifactKey::DirInterface {
                module: module_id,
                profile: profile_id,
            };
            let build_key = BuildKey::Artifact(artifact_key.clone());
            let dependency = self.build_dependency_for_key(&build_key);

            self.program
                .artifacts
                .set_dir_interface(module_id, profile_id, dir);
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
        let mut dir = self
            .require_artifact_dir(destack_workspace::ArtifactKey::dir_interface(
                module, profile,
            ))
            .map_err(AnalyzeError::from)?;
        {
            let dir = Arc::make_mut(&mut dir);
            let mut infer_table =
                self.analyze_module_infer(dir, module, profile, module_version, profile_version)?;
            self.analyze_module_solve(
                dir,
                infer_table.as_mut(),
                module,
                profile,
                module_version,
                profile_version,
            )?;
            self.analyze_module_commit(
                dir,
                infer_table.as_mut(),
                module,
                profile,
                module_version,
                profile_version,
            )?;
            self.analyze_module_capture(dir, module, profile, module_version, profile_version)?;
            self.analyze_module_validate(dir, module, profile, module_version, profile_version)?;
        }

        if self.is_code_module(module) {
            self.stats.record_analyze();
        }

        self.program
            .artifacts
            .set_dir_analyzed(module, profile, dir);

        Ok(())
    }
}
