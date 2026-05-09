use crate::{Compiler, CompilerError, CompilerResult, GenerateError, GenerateWarning};
use destack_artifact::ModuleOutput;
use destack_codegen_native::{CodegenCraneliftError, CodegenCraneliftWarning};
use destack_mir as mir;
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderContext, Target};

use super::GenerateState;

impl Compiler {
    /// Generate one binary module output through the native backend.
    pub(super) fn generate_binary_module_output(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ModuleOutput> {
        // construct target identity from the module package
        let module = self.module(context.revision(), module_id);
        let package_id = module.package_id;
        let target_id = TargetId::new(package_id, &target.name);

        // require the optimized MIR state
        self.require_mir_optimized(context, module_id, profile, &target_id)
            .map_err(CompilerError::from)?;
        let mir_optimized = self
            .mir_optimized(context, module_id, profile, &target_id)
            .map_err(CompilerError::from)?;
        let mir_lowered = self
            .mir_lowered(context, module_id, profile, &target_id)
            .map_err(CompilerError::from)?;
        let state = GenerateState::new(module_id, &mir_optimized.tree);

        // generate one binary output through the current backend
        let (artifact, warnings, errors) = destack_codegen_native::BinaryOutputGenerator::new(
            module.clone(),
            self.repository.string_pool().clone(),
            Some(mir_optimized.clone()),
            Some(mir_lowered),
            target,
        )
        .generate()
        .map_err(|error| Self::map_binary_generate_error(&state, error))?;
        // map backend diagnostics into compiler diagnostics
        for warning in warnings {
            let warning = Self::map_binary_generate_warning(&state, warning);
            self.emit_diagnostic(context, warning)?;
        }
        for error in errors {
            let error = Self::map_binary_generate_error(&state, error);
            self.emit_diagnostic(context, error)?;
        }

        Ok(ModuleOutput::Binary(Box::new(artifact)))
    }

    /// Map one binary backend error to a compiler error.
    fn map_binary_generate_error(
        state: &GenerateState<'_, mir::Tree>,
        error: CodegenCraneliftError,
    ) -> GenerateError {
        match error {
            CodegenCraneliftError::UnsupportedTarget { triple, .. } => {
                GenerateError::UnsupportedTarget {
                    anchor: (state.module_id).into(),
                    module: state.module_id,
                    target: triple,
                }
            }
            CodegenCraneliftError::FunctionNotFound { name, .. } => {
                GenerateError::UnresolvedFunction {
                    anchor: (state.module_id).into(),
                    module: state.module_id,
                    name,
                }
            }
            CodegenCraneliftError::Internal { message } => GenerateError::Internal {
                anchor: (state.module_id).into(),
                module: state.module_id,
                message,
            },
            CodegenCraneliftError::UnsupportedType { node, .. } => GenerateError::UnsupportedType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenCraneliftError::MissingType { node, .. } => GenerateError::MissingType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenCraneliftError::UnsupportedInstruction { node, .. } => {
                GenerateError::UnsupportedConstruct {
                    anchor: state.anchor(node),
                    module: state.module_id,
                    message: "unsupported instruction".to_string(),
                }
            }
            CodegenCraneliftError::OutOfBounds { node, index, len } => GenerateError::OutOfBounds {
                anchor: state.anchor(node),
                module: state.module_id,
                index,
                len,
            },
        }
    }

    /// Map one binary backend warning to a compiler warning.
    fn map_binary_generate_warning(
        state: &GenerateState<'_, mir::Tree>,
        warning: CodegenCraneliftWarning,
    ) -> GenerateWarning {
        match warning {
            CodegenCraneliftWarning::UnexpectedNode { node, .. } => {
                GenerateWarning::UnexpectedConstruct {
                    anchor: state.anchor(node),
                    module: state.module_id,
                }
            }
        }
    }
}
