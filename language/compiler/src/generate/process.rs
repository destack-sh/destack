use crate::timing::tags;
use crate::{BuildKey, BuildRequirementError, Compiler, GenerateError, GenerateResult};

use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{OutputFormat, OutputKey, ProfileId, TargetId};

impl Compiler {
    /// Build one module output.
    pub fn process_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> GenerateResult<()> {
        let module_stamp = self.module_stamp(module);
        let profile_stamp = self.profile_stamp(profile);
        self.ensure_module_profile_matches::<GenerateError>(
            module_stamp.id,
            module_stamp.version,
            profile_stamp.id,
            profile_stamp.version,
        )?;
        let _timing = self.timing_scope(tags::GENERATE_MODULE);
        self.generate_module(
            module_stamp.id,
            profile_stamp.id,
            module_stamp.version,
            profile_stamp.version,
            &target,
        )?;
        self.stats.record_generate();

        Ok(())
    }

    /// Generate code for a module.
    fn generate_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        target_id: &TargetId,
    ) -> GenerateResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<GenerateError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;

        // look up target from module's package
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            package.targets.get(target_id).cloned()
        };

        let target = target.ok_or_else(|| GenerateError::Internal {
            module: module_id,
            message: format!("target '{}' not found", target_id.name),
        })?;

        let resolved_profile = self
            .program
            .profile_id_for_target(module_id, target_id)
            .ok_or_else(|| GenerateError::Internal {
                module: module_id,
                message: format!("profile not found for target '{}'", target_id.name),
            })?;
        if resolved_profile != profile {
            return Ok(());
        }

        self.require_dir_patched(module_id, profile)?;

        // dispatch based on output format
        match target.output {
            OutputFormat::Js | OutputFormat::Ts => self.generate_js(module_id, &target, profile),
            // native codegen path, enabled by feature
            #[cfg(feature = "native-codegen")]
            OutputFormat::Native | OutputFormat::Wasm => {
                self.generate_cranelift(module_id, &target, profile)
            }
            // native codegen path, disabled by feature
            #[cfg(not(feature = "native-codegen"))]
            OutputFormat::Native | OutputFormat::Wasm => Err(GenerateError::Internal {
                module: module_id,
                message: format!(
                    "native codegen is disabled: cannot generate output '{:?}' for target '{}'",
                    target.output, target.name
                ),
            }),
        }
    }

    /// Require one module output build product.
    pub fn require_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), BuildRequirementError> {
        let _ = profile;
        self.require_build_key(BuildKey::Output(OutputKey::module(module, target.clone())))
    }
}
