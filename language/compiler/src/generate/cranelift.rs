use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning};
use destack_artifact::{ArtifactKey, ModuleOutput};
use destack_codegen_native::{CodegenCraneliftError, CodegenCraneliftWarning};
use destack_dir::{AnchoredGlobalNodeId, LocalNodeIdAny};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Target, TargetId};

impl Compiler {
    /// Generate native/WASM code for a module using Cranelift.
    pub(super) fn generate_cranelift(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
    ) -> GenerateResult<()> {
        // construct target id from module's package
        let module = self.program.modules.get(module_id);
        let package_id = module.package_id;
        let target_id = TargetId::new(package_id, &target.name);

        // require module to be optimized
        self.require_mir_optimized(module_id, profile, &target_id)?;

        // generate artifact
        let output = destack_codegen_native::generate_module(
            self.program.clone(),
            self.artifacts.clone(),
            module_id,
            target,
        )
        .map_err(|e| self.map_cranelift_error(module_id, &target.name, profile, e))?;
        self.artifacts.publish(
            ArtifactKey::module_output(module_id, target_id.clone()),
            ModuleOutput::new(target.emit, output.entries),
        );

        // map warnings/errors
        for warning in output.warnings {
            let warning = self.map_cranelift_warning(module_id, &target.name, profile, warning);
            self.warning(warning);
        }
        for error in output.errors {
            let error = self.map_cranelift_error(module_id, &target.name, profile, error);
            self.error(error);
        }

        Ok(())
    }

    /// Map a Cranelift error to a compiler error.
    fn map_cranelift_error(
        &self,
        module_id: ModuleId,
        target_name: &str,
        profile: ProfileId,
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
                node: self.get_dir_node_id(module_id, target_name, profile, node),
            },
            CodegenCraneliftError::MissingType { node, .. } => GenerateError::MissingType {
                module: module_id,
                node: self.get_dir_node_id(module_id, target_name, profile, node),
            },
            CodegenCraneliftError::UnsupportedInstruction { node, .. } => {
                GenerateError::UnsupportedConstruct {
                    module: module_id,
                    node: self.get_dir_node_id(module_id, target_name, profile, node),
                }
            }
            CodegenCraneliftError::OutOfBounds { node, index, len } => GenerateError::OutOfBounds {
                module: module_id,
                node: self.get_dir_node_id(module_id, target_name, profile, node),
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
        profile: ProfileId,
        warning: CodegenCraneliftWarning,
    ) -> GenerateWarning {
        match warning {
            CodegenCraneliftWarning::UnexpectedNode { node, .. } => {
                GenerateWarning::UnexpectedConstruct {
                    module: module_id,
                    node: self.get_dir_node_id(module_id, target_name, profile, node),
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
        profile: ProfileId,
        mir_node: destack_mir::LocalNodeIdAny,
    ) -> Option<AnchoredGlobalNodeId> {
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let target_id = TargetId::new(module.package_id, target_name);
        let dir_node_id =
            if let Some(mir) = self.artifacts.mir_optimized(module_id, profile, &target_id) {
                mir.tree.get_source(mir_node.id)?
            } else if let Some(mir) = self.artifacts.mir_base(module_id, profile, &target_id) {
                mir.tree.get_source(mir_node.id)?
            } else {
                panic!("code generation requires MIR artifact");
            };
        let dir = self.artifacts.dir_patched(module_id, profile)?;
        let dir_node_type = dir.tree.get_node_type(dir_node_id);

        let node = LocalNodeIdAny {
            id: dir_node_id,
            ty: dir_node_type,
        };
        Some(node.into_anchored(module_id, Some(profile)))
    }
}
