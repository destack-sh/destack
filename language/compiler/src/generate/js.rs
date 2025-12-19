use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning};
use destack_codegen_js::{CodegenJsError, CodegenJsWarning};
use destack_source::ModuleId;
use destack_workspace::Target;

impl Compiler {
    /// Generate JS/TS code for a module.
    pub(super) fn generate_js(&self, module_id: ModuleId, target: &Target) -> GenerateResult<()> {
        self.require_elaborate_module(module_id)?;

        // generate artifact
        let output = destack_codegen_js::generate_module(self.program.clone(), module_id, target)
            .map_err(|e| Self::map_js_error(module_id, e))?;
        for artifact in output.artifacts {
            self.program.artifacts.insert(artifact);
        }

        // map warnings/errors
        for warning in output.warnings {
            let warning = Self::map_js_warning(module_id, warning);
            self.warning(warning);
        }
        for error in output.errors {
            let error = Self::map_js_error(module_id, error);
            self.error(error);
        }


        Ok(())
    }

    /// Map a JS error to a compiler error.
    fn map_js_error(module_id: ModuleId, error: CodegenJsError) -> GenerateError {
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
                    node: Some(node),
                }
            }
            CodegenJsError::UnexpectedNode { node, .. } => GenerateError::UnexpectedConstruct {
                module: module_id,
                node: Some(node),
            },
            CodegenJsError::UnresolvedNode { node, .. } => GenerateError::UnresolvedConstruct {
                module: module_id,
                node: Some(node),
            },
            CodegenJsError::MissingType { node, .. } => GenerateError::MissingType {
                module: module_id,
                node: Some(node),
            },
        }
    }

    /// Map a JS warning to a compiler warning.
    fn map_js_warning(module_id: ModuleId, warning: CodegenJsWarning) -> GenerateWarning {
        match warning {
            CodegenJsWarning::ImpreciseType { node } => GenerateWarning::ImpreciseType {
                module: module_id,
                node: Some(node),
            },
            CodegenJsWarning::UnexpectedNode { node, .. } => GenerateWarning::UnexpectedConstruct {
                module: module_id,
                node: Some(node),
            },
            CodegenJsWarning::ExpectedStatement { node } => GenerateWarning::UnexpectedConstruct {
                module: module_id,
                node: Some(node),
            },
        }
    }
}
