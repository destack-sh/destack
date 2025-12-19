use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning};
use destack_codegen_js::{CodegenJsError, CodegenJsWarning};
use destack_source::ModuleId;
use destack_workspace::Target;

impl Compiler {
    /// Generate JS/TS code for a module.
    pub(super) fn generate_js(&self, module_id: ModuleId, target: &Target) -> GenerateResult<()> {
        // yield to Elaborate if not ready
        self.require_elaborate_module(module_id)?;

        // dispatch to JS codegen
        let output = destack_codegen_js::generate_module(self.program.clone(), module_id, target)
            .map_err(|e| Self::map_js_error(module_id, e))?;

        // map and report warnings
        for warning in output.warnings {
            let warning = Self::map_js_warning(warning);
            self.warning(warning);
        }

        // map and report errors (may be suppressed)
        for error in output.errors {
            let error = Self::map_js_error(module_id, error);
            self.error(error);
        }

        // store artifacts in registry
        for artifact in output.artifacts {
            self.program.artifacts.insert(artifact);
        }

        Ok(())
    }

    /// Map a JS codegen error to a GenerateError.
    fn map_js_error(module_id: ModuleId, error: CodegenJsError) -> GenerateError {
        match error {
            CodegenJsError::UnsupportedTarget { format, .. } => GenerateError::UnsupportedTarget {
                node: Self::placeholder_node(module_id),
                target: format,
            },
            CodegenJsError::UnsupportedConstruct { node, .. } => {
                GenerateError::UnsupportedConstruct { node }
            }
            CodegenJsError::UnexpectedNode { node, .. } => {
                GenerateError::UnexpectedConstruct { node }
            }
            CodegenJsError::UnresolvedNode { node, .. } => {
                GenerateError::UnresolvedConstruct { node }
            }
            CodegenJsError::MissingType { node, .. } => GenerateError::MissingType { node },
            CodegenJsError::Internal { message } => GenerateError::Internal {
                node: Self::placeholder_node(module_id),
                message,
            },
        }
    }

    /// Map a JS codegen warning to a GenerateWarning.
    fn map_js_warning(warning: CodegenJsWarning) -> GenerateWarning {
        match warning {
            CodegenJsWarning::ImpreciseType { node } => GenerateWarning::ImpreciseType { node },
            CodegenJsWarning::UnexpectedNode { node, .. } => {
                GenerateWarning::UnexpectedConstruct { node }
            }
            CodegenJsWarning::ExpectedStatement { node } => {
                GenerateWarning::UnexpectedConstruct { node }
            }
        }
    }
}
