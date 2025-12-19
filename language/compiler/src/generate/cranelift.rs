use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning};
use destack_codegen_cranelift::{CodegenCraneliftError, CodegenCraneliftWarning};
use destack_dir::{GlobalNodeIdAny, LocalNodeIdAny, NodeType};
use destack_source::ModuleId;
use destack_workspace::Target;

impl Compiler {
    /// Generate native/WASM code for a module using Cranelift.
    pub(super) fn generate_cranelift(
        &self,
        module_id: ModuleId,
        target: &Target,
    ) -> GenerateResult<()> {
        // yield to Optimize if not ready
        self.require_optimize(module_id)?;

        // dispatch to Cranelift codegen
        let registry_next_id = || self.program.artifacts.next_id();
        let output = destack_codegen_cranelift::generate_module(
            self.program.clone(),
            module_id,
            target,
            registry_next_id,
        )
        .map_err(|e| Self::map_cranelift_error(module_id, e))?;

        // map and report warnings
        for warning in output.warnings {
            let warning = Self::map_cranelift_warning(module_id, warning);
            self.warning(warning);
        }

        // map and report errors (may be suppressed)
        for error in output.errors {
            let error = Self::map_cranelift_error(module_id, error);
            self.error(error);
        }

        // store artifacts in registry
        for artifact in output.artifacts {
            self.program.artifacts.insert(artifact);
        }

        Ok(())
    }

    /// Map a Cranelift codegen error to a GenerateError.
    fn map_cranelift_error(module_id: ModuleId, error: CodegenCraneliftError) -> GenerateError {
        match error {
            CodegenCraneliftError::UnsupportedTarget { triple, .. } => {
                GenerateError::UnsupportedTarget {
                    node: Self::placeholder_node(module_id),
                    target: triple,
                }
            }
            CodegenCraneliftError::UnsupportedType { node, .. } => GenerateError::UnsupportedType {
                node: Self::mir_to_global_node(module_id, node),
            },
            CodegenCraneliftError::MissingType { node, .. } => GenerateError::MissingType {
                node: Self::mir_to_global_node(module_id, node),
            },
            CodegenCraneliftError::UnsupportedInstruction { node, .. } => {
                GenerateError::UnsupportedConstruct {
                    node: Self::mir_to_global_node(module_id, node),
                }
            }
            CodegenCraneliftError::FunctionNotFound { name, .. } => {
                GenerateError::UnresolvedFunction {
                    node: Self::placeholder_node(module_id),
                    name,
                }
            }
            CodegenCraneliftError::OutOfBounds { node, index, len } => GenerateError::OutOfBounds {
                node: Self::mir_to_global_node(module_id, node),
                index,
                len,
            },
            CodegenCraneliftError::Internal { message } => GenerateError::Internal {
                node: Self::placeholder_node(module_id),
                message,
            },
        }
    }

    /// Map a Cranelift codegen warning to a GenerateWarning.
    ///
    /// Currently Cranelift warnings don't map to any GenerateWarning variants,
    /// so we map them to UnexpectedConstruct as a catch-all.
    fn map_cranelift_warning(
        module_id: ModuleId,
        warning: CodegenCraneliftWarning,
    ) -> GenerateWarning {
        match warning {
            CodegenCraneliftWarning::UnexpectedNode { node, .. } => {
                GenerateWarning::UnexpectedConstruct {
                    node: Self::mir_to_global_node(module_id, node),
                }
            }
        }
    }

    /// Convert a MIR local node id to a global node id.
    /// nocheckin #Broken: revisit generate node mapping
    fn mir_to_global_node(
        module_id: ModuleId,
        node: destack_mir::LocalNodeIdAny,
    ) -> GlobalNodeIdAny {
        GlobalNodeIdAny::new(
            module_id,
            LocalNodeIdAny {
                id: node.id,
                ty: NodeType::Expression,
            },
        )
    }
}
