use std::sync::Arc;

use destack_artifact::{EmitFormat, MirLowered, MirOptimized, SourceMap};
use destack_core::StringPool;
use destack_repository::{Module, Target};
use destack_source::FileType;

use crate::{CodegenCraneliftError, CodegenCraneliftResult};

/// One generator for native module outputs.
#[derive(Debug)]
pub struct NativeOutputGenerator<'a> {
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

/// One native output payload before repository content interning.
#[derive(Debug, Clone)]
pub struct NativeOutputBytes {
    /// The emitted output file type.
    pub file_type: FileType,
    /// The emitted native payload bytes.
    pub bytes: Vec<u8>,
    /// The emitted source map payload when one exists.
    pub source_map: Option<SourceMap>,
}

impl NativeOutputBytes {
    /// Create one native output payload.
    pub fn new(file_type: FileType, bytes: Vec<u8>, source_map: Option<SourceMap>) -> Self {
        Self {
            file_type,
            bytes,
            source_map,
        }
    }
}

impl<'a> NativeOutputGenerator<'a> {
    /// Create one native output generator.
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

    /// Generate one native output.
    pub fn generate(
        self,
    ) -> CodegenCraneliftResult<(NativeOutputBytes, Vec<CodegenCraneliftError>)> {
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
            backend.compile_module(&mir.tree, &mir.layouts, self.strings.as_ref(), name)?
        } else if let Some(mir) = self.mir_lowered.as_ref() {
            backend.compile_module(&mir.tree, &mir.layouts, self.strings.as_ref(), name)?
        } else {
            panic!("codegen requires committed MIR artifact");
        };

        let output = match self.target.emit {
            EmitFormat::Native => {
                NativeOutputBytes::new(FileType::Object, compile_output.bytes, None)
            }
            EmitFormat::Wasm => NativeOutputBytes::new(FileType::Wasm, compile_output.bytes, None),
            _ => unreachable!(),
        };

        Ok((output, compile_output.errors))
    }
}
