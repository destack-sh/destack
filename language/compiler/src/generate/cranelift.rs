use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning};
use destack_codegen_cranelift::{CodegenCraneliftError, CodegenCraneliftWarning};
use destack_dir::{GlobalNodeIdAny, LocalNodeIdAny};
use destack_source::ModuleId;
use destack_workspace::Target;

impl Compiler {
    /// Generate native/WASM code for a module using Cranelift.
    pub(super) fn generate_cranelift(
        &self,
        module_id: ModuleId,
        target: &Target,
    ) -> GenerateResult<()> {
        self.require_optimize(module_id)?;

        // generate artifact
        let registry_next_id = || self.program.artifacts.next_id();
        let output = destack_codegen_cranelift::generate_module(
            self.program.clone(),
            module_id,
            target,
            registry_next_id,
        )
        .map_err(|e| self.map_cranelift_error(module_id, &target.name, e))?;
        for artifact in output.artifacts {
            self.program.artifacts.insert(artifact);
        }

        // map warnings/errors
        for warning in output.warnings {
            let warning = self.map_cranelift_warning(module_id, &target.name, warning);
            self.warning(warning);
        }
        for error in output.errors {
            let error = self.map_cranelift_error(module_id, &target.name, error);
            self.error(error);
        }

        Ok(())
    }

    /// Map a Cranelift error to a compiler error.
    fn map_cranelift_error(
        &self,
        module_id: ModuleId,
        target_name: &str,
        error: CodegenCraneliftError,
    ) -> GenerateError {
        match error {
            CodegenCraneliftError::UnsupportedTarget { triple, .. } => {
                GenerateError::UnsupportedTarget {
                    module: module_id,
                    target: triple,
                }
            }
            CodegenCraneliftError::FunctionNotFound { name, .. } => {
                GenerateError::UnresolvedFunction {
                    module: module_id,
                    name,
                }
            }
            CodegenCraneliftError::Internal { message } => GenerateError::Internal {
                module: module_id,
                message,
            },
            CodegenCraneliftError::UnsupportedType { node, .. } => GenerateError::UnsupportedType {
                module: module_id,
                node: self.get_dir_node_id(module_id, target_name, node),
            },
            CodegenCraneliftError::MissingType { node, .. } => GenerateError::MissingType {
                module: module_id,
                node: self.get_dir_node_id(module_id, target_name, node),
            },
            CodegenCraneliftError::UnsupportedInstruction { node, .. } => {
                GenerateError::UnsupportedConstruct {
                    module: module_id,
                    node: self.get_dir_node_id(module_id, target_name, node),
                }
            }
            CodegenCraneliftError::OutOfBounds { node, index, len } => GenerateError::OutOfBounds {
                module: module_id,
                node: self.get_dir_node_id(module_id, target_name, node),
                index,
                len,
            },
        }
    }

    /// Map a Cranelift warning to a compiler warning.
    fn map_cranelift_warning(
        &self,
        module_id: ModuleId,
        target_name: &str,
        warning: CodegenCraneliftWarning,
    ) -> GenerateWarning {
        match warning {
            CodegenCraneliftWarning::UnexpectedNode { node, .. } => {
                GenerateWarning::UnexpectedConstruct {
                    module: module_id,
                    node: self.get_dir_node_id(module_id, target_name, node),
                }
            }
        }
    }

    /// Look up source DIR node from MIR node via source tracking.
    /// Returns None if MIR node is synthesized (no source).
    fn get_dir_node_id(
        &self,
        module_id: ModuleId,
        target_name: &str,
        mir_node: destack_mir::LocalNodeIdAny,
    ) -> Option<GlobalNodeIdAny> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mir_tree = module.mir(target_name).tree.read();

        let dir_node_id = mir_tree.get_source(mir_node.id)?;
        let dir_tree = module.dir().tree.read();
        let dir_node_type = dir_tree.get_node_type(dir_node_id);

        Some(GlobalNodeIdAny::new(
            module_id,
            LocalNodeIdAny {
                id: dir_node_id,
                ty: dir_node_type,
            },
        ))
    }
}
