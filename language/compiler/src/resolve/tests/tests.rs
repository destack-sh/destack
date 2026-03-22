use destack_source::ModuleId;
use destack_workspace::{EmitFormat, Runtime};

pub(super) use crate::{Compiler, TestProgram, assert_node, assert_string};

impl TestProgram {
    /// Set the default profile for one module to the given runtime, emit format, and libraries.
    pub(crate) fn set_module_profile(
        &mut self,
        module_id: ModuleId,
        runtime: Runtime,
        emit: EmitFormat,
        libs: &[&str],
    ) {
        let default_profile = self.program.profile(self.default_profile_id(module_id));
        let mut key = default_profile.key.clone();
        key.emit = emit;
        key.runtime = runtime;
        key.lib = libs.iter().map(|lib| (*lib).to_string()).collect();

        let profile_id = self.program.profiles.get_or_create(key);
        self.default_profile_override = Some(profile_id);
    }

    /// Set noInternalImport policy for the module package config.
    pub(crate) fn set_module_no_internal_import_policy(&self, module_id: ModuleId, policy: &str) {
        self.apply_destack_config(
            module_id,
            &format!(
                r#"
{{
  "compilerOptions": {{
    "noInternalImport": "{policy}"
  }}
}}
"#
            ),
        );
    }
}
