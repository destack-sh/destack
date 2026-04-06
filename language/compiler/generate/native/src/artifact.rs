use std::sync::Arc;

use destack_artifact::{
    BinaryArtifact, EmitFormat, MirBase, MirOptimized, ObjectArtifact, WasmArtifact, WasmInterface,
};
use destack_workspace::{Module, Target};

use crate::{CodegenCraneliftError, CodegenCraneliftResult, CodegenCraneliftWarning};

/// One generator for binary module artifacts.
#[derive(Debug)]
pub struct BinaryArtifactGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// The current optimized MIR, when available.
    mir_optimized: Option<Arc<MirOptimized>>,
    /// The current base MIR fallback.
    mir_base: Option<Arc<MirBase>>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> BinaryArtifactGenerator<'a> {
    /// Create one binary artifact generator.
    pub fn new(
        module: Arc<Module>,
        mir_optimized: Option<Arc<MirOptimized>>,
        mir_base: Option<Arc<MirBase>>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            mir_optimized,
            mir_base,
            target,
        }
    }

    /// Generate one binary artifact.
    pub fn generate(
        self,
    ) -> CodegenCraneliftResult<(
        BinaryArtifact,
        Vec<CodegenCraneliftWarning>,
        Vec<CodegenCraneliftError>,
    )> {
        // validate target
        match self.target.emit {
            EmitFormat::Wasm | EmitFormat::Native => {}
            other => {
                return Err(CodegenCraneliftError::UnsupportedTarget {
                    triple: format!("{other:?}"),
                    message: Some("expected Wasm or Native".to_string()),
                });
            }
        }

        // create backend
        let backend = crate::CodegenCraneliftBackend::new(self.target)?;

        // get module and its MIR
        // compile
        let module = self.module.as_ref();
        let name = module.uri.last_segment().unwrap_or("module");
        let compile_output = if let Some(mir) = self.mir_optimized.as_ref() {
            backend.compile_module(&mir.tree, &mir.strings, name)?
        } else if let Some(mir) = self.mir_base.as_ref() {
            backend.compile_module(&mir.tree, &mir.strings, name)?
        } else {
            panic!("codegen requires committed MIR artifact");
        };

        let artifact = match self.target.emit {
            EmitFormat::Native => BinaryArtifact::Object(Box::new(ObjectArtifact {
                bytes: Arc::from(compile_output.bytes),
                debug: Vec::new(),
            })),
            EmitFormat::Wasm => BinaryArtifact::Wasm(Box::new(WasmArtifact {
                bytes: Arc::from(compile_output.bytes),
                interface: WasmInterface::default(),
                source_map: None,
            })),
            _ => unreachable!(),
        };

        Ok((artifact, compile_output.warnings, compile_output.errors))
    }
}
