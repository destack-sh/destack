use crate::{Compiler, GenerateError, GenerateResult, GenerateWarning, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_dir::{GlobalNodeIdAny, LocalNodeIdAny, NodeType};
use destack_source::ModuleId;
use destack_workspace::{OutputFormat, Target};

/// Task to generate code for a module into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Generate)]
pub enum GenerateTask {
    /// Generate a module for a specific target.
    #[task(code = 1, trace = "module={module} target={target}")]
    GenerateModule {
        /// The module to generate.
        module: ModuleId,
        /// The target name (looked up on the module's package).
        target: String,
    },
}

impl Compiler {
    /// Process a generate task.
    pub fn process_generate(&self, task: GenerateTask) -> GenerateResult<()> {
        match task {
            GenerateTask::GenerateModule { module, target } => {
                self.generate_module(module, &target)?;
            }
        }
        Ok(())
    }

    /// Generate code for a module.
    fn generate_module(&self, module_id: ModuleId, target_name: &str) -> GenerateResult<()> {
        // look up target from module's package
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            package.targets.get(target_name).cloned()
        };

        let target = target.ok_or_else(|| GenerateError::Internal {
            node: Self::placeholder_node(module_id),
            message: format!("target '{target_name}' not found"),
        })?;

        // dispatch based on output format
        match target.output {
            OutputFormat::Js | OutputFormat::Ts => self.generate_js(module_id, &target),
            OutputFormat::Native | OutputFormat::Wasm => {
                self.generate_cranelift(module_id, &target)
            }
        }
    }

    /// Generate JS/TS code for a module.
    fn generate_js(&self, module_id: ModuleId, target: &Target) -> GenerateResult<()> {
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

    /// Generate native/WASM code for a module using Cranelift.
    fn generate_cranelift(&self, module_id: ModuleId, target: &Target) -> GenerateResult<()> {
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

    /// Map a JS codegen error to a GenerateError.
    fn map_js_error(
        module_id: ModuleId,
        error: destack_codegen_js::CodegenJsError,
    ) -> GenerateError {
        use destack_codegen_js::CodegenJsError;

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

    /// Map a Cranelift codegen error to a GenerateError.
    fn map_cranelift_error(
        module_id: ModuleId,
        error: destack_codegen_cranelift::CodegenCraneliftError,
    ) -> GenerateError {
        use destack_codegen_cranelift::CodegenCraneliftError;

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

    /// Map a JS codegen warning to a GenerateWarning.
    fn map_js_warning(warning: destack_codegen_js::CodegenJsWarning) -> GenerateWarning {
        use destack_codegen_js::CodegenJsWarning;

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

    /// Map a Cranelift codegen warning to a GenerateWarning.
    ///
    /// Currently Cranelift warnings don't map to any GenerateWarning variants,
    /// so we map them to UnexpectedConstruct as a catch-all.
    fn map_cranelift_warning(
        module_id: ModuleId,
        warning: destack_codegen_cranelift::CodegenCraneliftWarning,
    ) -> GenerateWarning {
        use destack_codegen_cranelift::CodegenCraneliftWarning;

        match warning {
            CodegenCraneliftWarning::UnoptimizedCodePath { node, .. }
            | CodegenCraneliftWarning::PerformanceHint { node, .. } => {
                GenerateWarning::UnexpectedConstruct {
                    node: Self::mir_to_global_node(module_id, node),
                }
            }
        }
    }

    /// Convert a MIR local node id to a global node id.
    fn mir_to_global_node(
        module_id: ModuleId,
        node: destack_mir::LocalNodeIdAny,
    ) -> GlobalNodeIdAny {
        // MIR node types don't map 1:1 to DIR, use Expression as fallback
        GlobalNodeIdAny::new(
            module_id,
            LocalNodeIdAny {
                id: node.id,
                ty: NodeType::Expression,
            },
        )
    }

    /// Create a placeholder node for errors without a specific location.
    fn placeholder_node(module_id: ModuleId) -> GlobalNodeIdAny {
        GlobalNodeIdAny::new(
            module_id,
            LocalNodeIdAny {
                id: 0,
                ty: NodeType::Expression,
            },
        )
    }

    /// Ensure a module has been generated.
    pub fn require_generate_module(
        &self,
        module: ModuleId,
        target: &str,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(GenerateTask::GenerateModule {
            module,
            target: target.to_string(),
        })
    }
}
