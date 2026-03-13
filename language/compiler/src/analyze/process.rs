use crate::analyze::DirReadBoundary;
use crate::{AnalyzeError, AnalyzeResult, BuildProduct, Compiler};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Build declared DIR for one module.
    pub fn process_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<BuildProduct> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<AnalyzeError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        // transient declared builder
        let dir = self
            .require_artifact_dir_resolved(module, profile)
            .map_err(AnalyzeError::from)?;
        let (_, dir) = self.with_shared_transient_artifact_dir(
            module,
            profile,
            DirReadBoundary::Declared,
            dir,
            |dir| {
                self.analyze_module_declare(
                    dir.as_ref(),
                    module,
                    profile,
                    module_version,
                    profile_version,
                )
            },
        )?;

        Ok(BuildProduct::Dir(dir.to_data()))
    }

    /// Build interface DIR for one module.
    pub fn process_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<BuildProduct> {
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

        Ok(BuildProduct::DirInterfaceBatch(entries))
    }

    /// Build analyzed DIR for one module.
    pub fn process_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<BuildProduct> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<AnalyzeError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        // transient analyzed builder
        let dir = self
            .require_artifact_dir_for_boundary(module, profile, DirReadBoundary::Interface)
            .map_err(AnalyzeError::from)?;
        let (_, dir) = self.with_shared_transient_artifact_dir(
            module,
            profile,
            DirReadBoundary::Analyzed,
            dir,
            |dir| -> AnalyzeResult<()> {
                let mut infer_table = self.analyze_module_infer(
                    dir.as_ref(),
                    module,
                    profile,
                    module_version,
                    profile_version,
                )?;
                self.analyze_module_solve(
                    dir.as_ref(),
                    infer_table.as_mut(),
                    module,
                    profile,
                    module_version,
                    profile_version,
                )?;
                self.analyze_module_commit(
                    dir.as_ref(),
                    infer_table.as_mut(),
                    module,
                    profile,
                    module_version,
                    profile_version,
                )?;
                self.analyze_module_capture(
                    dir.as_ref(),
                    module,
                    profile,
                    module_version,
                    profile_version,
                )?;
                self.analyze_module_validate(
                    dir.as_ref(),
                    module,
                    profile,
                    module_version,
                    profile_version,
                )?;

                Ok(())
            },
        )?;

        if self.is_code_module(module) {
            self.stats.record_analyze();
        }

        Ok(BuildProduct::Dir(dir.to_data()))
    }
}
