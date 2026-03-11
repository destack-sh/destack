use destack_workspace::ArtifactKey;

use crate::{BuildKey, BuildRequirementError, Compiler, ImportError, ImportResult};
use destack_source::ModuleId;

impl Compiler {
    /// Build the parsed syntax tree for one module.
    pub fn process_ast(&self, module: ModuleId) -> ImportResult<()> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.import_module_parse(module, module_version)?;

        Ok(())
    }

    /// Build base DIR for one module.
    pub fn process_dir_base(&self, module: ModuleId) -> ImportResult<()> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.require_ast(module)?;
        self.import_module_bind(module, module_version)?;
        self.import_module_desugar(module, module_version)?;
        self.import_module_validate(module, module_version)?;
        if self.is_code_module(module) {
            self.stats.record_bind();
        }

        Ok(())
    }

    /// Ensure a module AST exists.
    pub fn require_ast(&self, module: ModuleId) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::Ast { module }))
    }

    /// Ensure a module base DIR exists.
    pub fn require_dir_base(&self, module: ModuleId) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirBase { module }))
    }
}
