use crate::{Compiler, CompilerContext, GenerateError, GenerateResult, GenerateWarning};

use destack_artifact::{ArtifactKey, ModuleOutput};
use destack_codegen_js::{CodegenJsError, CodegenJsWarning};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Target};

impl Compiler {
    /// Generate one script module artifact.
    pub(super) fn generate_script_module_output(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> GenerateResult<()> {
        // require the patched module state
        self.require_dir_patched(context.revision(), module_id, profile)?;

        // snapshot module for this generate pass
        let module = context.module(module_id);
        let ast = self.ast(module_id).ok_or_else(|| GenerateError::Internal {
            module: module_id,
            message: "missing compiler AST artifact".to_string(),
        })?;
        let dir = self
            .dir_patched(module_id, profile)
            .ok_or_else(|| GenerateError::Internal {
                module: module_id,
                message: "missing compiler patched DIR artifact".to_string(),
            })?;

        // generate one script artifact through the current backend
        let (artifact, warnings, errors) = destack_codegen_js::ScriptArtifactGenerator::new(
            module.clone(),
            ast,
            dir,
            self.repository.string_pool().clone(),
            target,
        )
        .generate()
        .map_err(|error| Self::map_script_generate_error(module_id, profile, error))?;
        let target_id = self
            .repository
            .intern_target_id(module.package_id, &target.name);
        context.publish_artifact(
            ArtifactKey::module_output(module_id, target_id),
            ModuleOutput::Script(Box::new(artifact)),
            |store, version, payload| store.publish_module_output(version, payload),
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
