use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

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
        self.analyze_module_declare(module, profile, module_version, profile_version)?;

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
        self.analyze_module_interface(module, profile, module_version, profile_version)?;

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
        self.analyze_module_infer(module, profile, module_version, profile_version)?;
        self.analyze_module_solve(module, profile, module_version, profile_version)?;
        self.analyze_module_commit(module, profile, module_version, profile_version)?;
        self.analyze_module_capture(module, profile, module_version, profile_version)?;
        self.analyze_module_validate(module, profile, module_version, profile_version)?;
        if self.is_code_module(module) {
            self.stats.record_analyze();
        }

        Ok(())
    }
}
