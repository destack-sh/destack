use crate::{Compiler, CompilerError, CompilerResult, GenerateError, GenerateWarning};
use destack_artifact::ModuleOutput;
use destack_codegen_js::{CodegenJsError, CodegenJsWarning};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext, Target};
use destack_source::ModuleId;

use super::GenerateState;

impl Compiler {
    /// Generate one script module output.
    pub(super) fn generate_script_module_output(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ModuleOutput> {
        // snapshot module for this generate pass
        let artifacts = self.artifact_reader(context);
        let module = self.module(context.revision(), module_id)?;
        let parsed = artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module_id, profile)
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .dir_imported(module_id, profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module_id, profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module_id, profile)
            .map_err(CompilerError::from)?;

        // generate one script output through the current backend
        let state = GenerateState::new(module_id, &parsed.tree);
        let (artifact, warnings, errors) = destack_codegen_js::ScriptOutputGenerator::new(
            module.clone(),
            parsed.clone(),
            bound.clone(),
            imported,
            expanded,
            checked,
            self.repository.string_pool().clone(),
            target,
        )
        .generate()
        .map_err(|error| Self::map_script_generate_error(&state, error))?;
        // map backend diagnostics into compiler diagnostics
        for warning in warnings {
            let warning = Self::map_script_generate_warning(&state, warning);
            self.emit_diagnostic(context, warning)?;
        }
        for error in errors {
            let error = Self::map_script_generate_error(&state, error);
            self.emit_diagnostic(context, error)?;
        }

        Ok(ModuleOutput::Script(Box::new(artifact)))
    }

    /// Map one script backend error to a compiler error.
    fn map_script_generate_error(
        state: &GenerateState<'_, dir::Tree>,
        error: CodegenJsError,
    ) -> GenerateError {
        match error {
            CodegenJsError::UnsupportedTarget { format, .. } => GenerateError::UnsupportedTarget {
                anchor: (state.module_id).into(),
                module: state.module_id,
                target: format,
            },
            CodegenJsError::Internal { message } => GenerateError::Internal {
                anchor: (state.module_id).into(),
                module: state.module_id,
                message,
            },
            CodegenJsError::UnsupportedConstruct { node, message } => {
                GenerateError::UnsupportedConstruct {
                    anchor: state.anchor(node),
                    module: state.module_id,
                    message: message
                        .unwrap_or_else(|| format!("unsupported {}", node.local_id.ty.name())),
                }
            }
            CodegenJsError::UnexpectedNode {
                node,
                wanted,
                message,
            } => GenerateError::UnexpectedConstruct {
                anchor: state.anchor(node),
                module: state.module_id,
                message: message.unwrap_or_else(|| {
                    format!(
                        "unexpected {} (wanted {})",
                        node.local_id.ty.name(),
                        wanted.name()
                    )
                }),
            },
            CodegenJsError::UnresolvedNode { node, .. } => GenerateError::UnresolvedConstruct {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenJsError::MissingType { node, .. } => GenerateError::MissingType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
        }
    }

    /// Map one script backend warning to a compiler warning.
    fn map_script_generate_warning(
        state: &GenerateState<'_, dir::Tree>,
        warning: CodegenJsWarning,
    ) -> GenerateWarning {
        match warning {
            CodegenJsWarning::ImpreciseType { node } => GenerateWarning::ImpreciseType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenJsWarning::UnexpectedNode { node, .. } => GenerateWarning::UnexpectedConstruct {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenJsWarning::ExpectedStatement { node } => GenerateWarning::UnexpectedConstruct {
                anchor: state.anchor(node),
                module: state.module_id,
            },
        }
    }
}
