use std::str::FromStr;
use std::sync::Arc;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use destack_artifact::EmitFormat;
use destack_codegen_lib::CodegenBackend;
use destack_core::StringPool;
use destack_mir as mir;
use destack_repository::{Mode, OptimizeLevel, Target};
use target_lexicon::Triple;

use crate::lower::{ModuleLowerOutput, ModuleLowerer};
use crate::{CodegenCraneliftError, CodegenCraneliftResult};

/// Cranelift-based code generation backend.
///
/// Compiles MIR to native code or WebAssembly using Cranelift.
pub struct CodegenCraneliftBackend {
    /// The target ISA configuration.
    isa: Arc<dyn TargetIsa>,
    /// Whether to include debug info.
    #[allow(dead_code)]
    debug: bool,
}

impl std::fmt::Debug for CodegenCraneliftBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CraneliftCodegenBackend")
            .field("isa", &self.isa.name())
            .field("debug", &self.debug)
            .finish()
    }
}

impl CodegenCraneliftBackend {
    /// Create a new backend for the given target configuration.
    pub fn new(target: &Target) -> CodegenCraneliftResult<Self> {
        let triple = Self::target_triple(target)?;
        let isa = Self::create_isa(&triple, target)?;

        Ok(Self {
            isa,
            debug: target.modes.iter().any(|mode| mode == Mode::DEBUG.name),
        })
    }

    /// Create a backend for WebAssembly output.
    pub fn wasm() -> CodegenCraneliftResult<Self> {
        Self::new(&Target::wasm_js())
    }

    /// Create a backend for native output (host triple).
    pub fn native() -> CodegenCraneliftResult<Self> {
        Self::new(&Target::native())
    }

    /// Get the target triple for the given target configuration.
    fn target_triple(target: &Target) -> CodegenCraneliftResult<Triple> {
        match target.emit {
            EmitFormat::Wasm => Ok(Triple::from_str("wasm32-unknown-unknown").unwrap()),
            EmitFormat::Native => cranelift_native::builder()
                .map(|b| b.triple().clone())
                .map_err(|e| CodegenCraneliftError::Internal {
                    message: e.to_string(),
                }),
            _ => Err(CodegenCraneliftError::UnsupportedTarget {
                triple: format!("{:?}", target.emit),
                message: None,
            }),
        }
    }

    /// Create the target ISA from the triple and target configuration.
    fn create_isa(
        triple: &Triple,
        target: &Target,
    ) -> Result<Arc<dyn TargetIsa>, CodegenCraneliftError> {
        // configure
        let mut flags_builder = settings::builder();
        let opt_level = match target.optimize_level {
            OptimizeLevel::O0 => "none",
            OptimizeLevel::O1 => "speed",
            OptimizeLevel::O2 => "speed",
            OptimizeLevel::O3 => "speed_and_size",
            OptimizeLevel::O4 => "speed_and_size",
        };
        flags_builder
            .set("opt_level", opt_level)
            .map_err(|e| CodegenCraneliftError::Internal {
                message: e.to_string(),
            })?;

        // Enable PIC on AArch64 to use GOT-based relocations (Aarch64AdrGotPage21)
        // instead of direct PC-relative (Aarch64AdrPrelPgHi21) which cranelift-object
        // doesn't support.
        if triple.architecture
            == target_lexicon::Architecture::Aarch64(target_lexicon::Aarch64Architecture::Aarch64)
        {
            flags_builder
                .set("is_pic", "true")
                .map_err(|e| CodegenCraneliftError::Internal {
                    message: e.to_string(),
                })?;
        }

        let flags = settings::Flags::new(flags_builder);

        // create isa
        let isa_builder = cranelift_codegen::isa::lookup(triple.clone()).map_err(|e| {
            CodegenCraneliftError::UnsupportedTarget {
                triple: e.to_string(),
                message: None,
            }
        })?;
        isa_builder
            .finish(flags)
            .map_err(|e| CodegenCraneliftError::Internal {
                message: e.to_string(),
            })
    }

    /// Get the target ISA.
    pub fn isa(&self) -> &dyn TargetIsa {
        self.isa.as_ref()
    }

    /// Get the pointer byte size for this target.
    pub fn pointer_size_bytes(&self) -> u8 {
        self.isa.pointer_bytes()
    }

    /// Compile a MIR module to bytes and warnings.
    ///
    /// For WASM targets, returns a `.wasm` module.
    /// For native targets, returns an object file.
    pub(crate) fn compile_module(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> Result<ModuleLowerOutput, CodegenCraneliftError> {
        let mut lowerer = ModuleLowerer::new(self.isa.clone(), strings, name);
        lowerer.lower_module(tree)?;
        lowerer.finish()
    }

    /// Compile a MIR module and return Cranelift IR text format.
    pub fn compile_to_clif(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> CodegenCraneliftResult<String> {
        let mut lowerer = ModuleLowerer::new(self.isa.clone(), strings, name);
        lowerer.lower_module(tree)?;
        lowerer.as_clif_string()
    }
}

impl CodegenBackend for CodegenCraneliftBackend {
    fn name(&self) -> &'static str {
        "cranelift"
    }

    fn supports_target(&self, target: &Target) -> bool {
        target.uses_native_generate_pipeline()
    }
}
