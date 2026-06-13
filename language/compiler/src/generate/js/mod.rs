mod diagnostic;
mod lower;
mod output;
mod print;

pub(crate) use destack_js::*;
pub(crate) use diagnostic::*;
pub(crate) use lower::*;
pub(crate) use output::*;
pub(crate) use print::*;

#[cfg(test)]
pub(crate) mod tests;

use crate::{Compiler, CompilerError, CompilerResult, GenerateError};
use destack_artifact::ModuleOutput;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, Target};
use destack_source::ModuleId;

use super::GenerateState;

impl Compiler {
    /// Generate one JS module output.
    pub(super) fn generate_js_module_output(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<ModuleOutput> {
        // snapshot module for this generate pass
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

        // generate one JS output
        let state = GenerateState::new(module_id, &parsed.tree);
        let (artifact, errors) = JsOutputGenerator::new(
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
        .map_err(|error| Self::map_js_generate_error(&state, error))?;
        // map JS diagnostics into compiler diagnostics
        for error in errors {
            let error = Self::map_js_generate_error(&state, error);
            self.emit_diagnostic(context, error)?;
        }

        Ok(ModuleOutput::Js(Box::new(artifact)))
    }

    /// Map one JS generation error to a compiler error.
    fn map_js_generate_error(
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
            CodegenJsError::MissingType { node, .. } => GenerateError::MissingType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
        }
    }
}
