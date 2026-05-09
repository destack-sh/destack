use std::sync::Arc;

use destack_artifact::{BinaryOutput, EmitFormat, MirLowered, MirOptimized};
use destack_core::StringPool;
use destack_workspace::{Module, Target};

use crate::{CodegenCraneliftError, CodegenCraneliftResult, CodegenCraneliftWarning};

/// One generator for binary module outputs.
#[derive(Debug)]
pub struct BinaryOutputGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// Shared strings referenced by MIR.
    strings: Arc<StringPool>,
    /// The current optimized MIR, when available.
    mir_optimized: Option<Arc<MirOptimized>>,
    /// The current lowered MIR fallback.
    mir_lowered: Option<Arc<MirLowered>>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> BinaryOutputGenerator<'a> {
    /// Create one binary output generator.
    pub fn new(
        module: Arc<Module>,
        strings: Arc<StringPool>,
        mir_optimized: Option<Arc<MirOptimized>>,
        mir_lowered: Option<Arc<MirLowered>>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            strings,
            mir_optimized,
            mir_lowered,
            target,
        }
    }

    /// Generate one binary output.
    pub fn generate(
        self,
    ) -> CodegenCraneliftResult<(
        BinaryOutput,
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

        // compile
        let module = self.module.as_ref();
        let name = module.uri.last_segment().unwrap_or("module");
        let compile_output = if let Some(mir) = self.mir_optimized.as_ref() {
            backend.compile_module(&mir.tree, self.strings.as_ref(), name)?
        } else if let Some(mir) = self.mir_lowered.as_ref() {
            backend.compile_module(&mir.tree, self.strings.as_ref(), name)?
        } else {
            panic!("codegen requires committed MIR artifact");
        };

        let artifact = match self.target.emit {
            EmitFormat::Native => BinaryOutput::object(compile_output.bytes),
            EmitFormat::Wasm => BinaryOutput::wasm(compile_output.bytes, None),
            _ => unreachable!(),
        };

        Ok((artifact, compile_output.warnings, compile_output.errors))
    }
}
