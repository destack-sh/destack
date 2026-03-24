use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning};

use destack_artifact::{ArtifactKey, ModuleArtifact};
use destack_codegen_js::{CodegenJsError, CodegenJsWarning};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Target, TargetId};

impl Compiler {
    /// Generate one script module artifact.
    pub(super) fn generate_script_module_artifact(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
    ) -> GenerateResult<()> {
        // require the elaborated module state
        self.require_dir_elaborated(module_id, profile)?;

        // generate one script artifact through the current backend
        let (artifact, warnings, errors) = destack_codegen_js::ScriptArtifactGenerator::new(
            self.program.clone(),
            self.artifacts.clone(),
            module_id,
            target,
            profile,
        )
        .generate()
        .map_err(|error| Self::map_script_generate_error(module_id, profile, error))?;
        let module = self.program.modules.get(module_id);
        let target_id = TargetId::new(module.package_id, &target.name);
        self.artifacts.publish(
            ArtifactKey::module_artifact(module_id, target_id),
            ModuleArtifact::Script(artifact),
        );

        // map backend diagnostics into compiler diagnostics
        for warning in warnings {
            let warning = Self::map_script_generate_warning(module_id, profile, warning);
            self.warning(warning);
        }
        for error in errors {
            let error = Self::map_script_generate_error(module_id, profile, error);
            self.error(error);
        }

        Ok(())
    }

    /// Map one script backend error to a compiler error.
    fn map_script_generate_error(
        module_id: ModuleId,
        profile: ProfileId,
        error: CodegenJsError,
    ) -> GenerateError {
        match error {
            CodegenJsError::UnsupportedTarget { format, .. } => GenerateError::UnsupportedTarget {
                module: module_id,
                target: format,
            },
            CodegenJsError::Internal { message } => GenerateError::Internal {
                module: module_id,
                message,
            },
            CodegenJsError::UnsupportedConstruct { node, .. } => {
                GenerateError::UnsupportedConstruct {
                    module: module_id,
                    node: Some(node.into_anchored(Some(profile))),
                }
            }
            CodegenJsError::UnexpectedNode { node, .. } => GenerateError::UnexpectedConstruct {
                module: module_id,
                node: Some(node.into_anchored(Some(profile))),
            },
            CodegenJsError::UnresolvedNode { node, .. } => GenerateError::UnresolvedConstruct {
                module: module_id,
                node: Some(node.into_anchored(Some(profile))),
            },
            CodegenJsError::MissingType { node, .. } => GenerateError::MissingType {
                module: module_id,
                node: Some(node.into_anchored(Some(profile))),
            },
        }
    }

    /// Map one script backend warning to a compiler warning.
    fn map_script_generate_warning(
        module_id: ModuleId,
        profile: ProfileId,
        warning: CodegenJsWarning,
    ) -> GenerateWarning {
        match warning {
            CodegenJsWarning::ImpreciseType { node } => GenerateWarning::ImpreciseType {
                module: module_id,
                node: Some(node.into_anchored(Some(profile))),
            },
            CodegenJsWarning::UnexpectedNode { node, .. } => GenerateWarning::UnexpectedConstruct {
                module: module_id,
                node: Some(node.into_anchored(Some(profile))),
            },
            CodegenJsWarning::ExpectedStatement { node } => GenerateWarning::UnexpectedConstruct {
                module: module_id,
                node: Some(node.into_anchored(Some(profile))),
            },
        }
    }
}
