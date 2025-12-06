use std::str::FromStr;
use std::sync::Arc;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use destack_codegen_lib::CodegenBackend;
use destack_mir::NodeTree;
use destack_source::ImmutableStringPool;
use destack_workspace::{OutputFormat, Target};
use target_lexicon::Triple;

use crate::CraneliftError;
use crate::lower::ModuleLowerer;

/// Cranelift-based code generation backend.
///
/// Compiles MIR to native code or WebAssembly using Cranelift.
pub struct CraneliftCodegenBackend {
    /// The target ISA configuration.
    isa: Arc<dyn TargetIsa>,
    /// Whether to include debug info.
    debug: bool,
}

impl std::fmt::Debug for CraneliftCodegenBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CraneliftCodegenBackend")
            .field("isa", &self.isa.name())
            .field("debug", &self.debug)
            .finish()
    }
}

impl CraneliftCodegenBackend {
    /// Create a new backend for the given target configuration.
    pub fn new(target: &Target) -> Result<Self, CraneliftError> {
        let triple = Self::target_triple(target)?;
        let isa = Self::create_isa(&triple, target)?;

        Ok(Self {
            isa,
            debug: target.debug,
        })
    }

    /// Create a backend for WebAssembly output.
    pub fn wasm() -> Result<Self, CraneliftError> {
        Self::new(&Target::wasm("wasm"))
    }

    /// Create a backend for native output (host triple).
    pub fn native() -> Result<Self, CraneliftError> {
        Self::new(&Target::native("native"))
    }

    /// Get the target triple for the given target configuration.
    fn target_triple(target: &Target) -> Result<Triple, CraneliftError> {
        match target.output {
            OutputFormat::Wasm => Ok(Triple::from_str("wasm32-unknown-unknown").unwrap()),
            OutputFormat::Native => cranelift_native::builder()
                .map(|b| b.triple().clone())
                .map_err(|e| CraneliftError::Internal {
                    message: e.to_string(),
                }),
            _ => Err(CraneliftError::UnsupportedTarget {
                triple: format!("{:?}", target.output),
            }),
        }
    }

    /// Create the target ISA from the triple and target configuration.
    fn create_isa(triple: &Triple, target: &Target) -> Result<Arc<dyn TargetIsa>, CraneliftError> {
        let mut flags_builder = settings::builder();

        // configure optimization level
        let opt_level = if target.optimize {
            match target.optimize_level {
                destack_workspace::OptimizeLevel::O0 => "none",
                destack_workspace::OptimizeLevel::O1 => "speed",
                destack_workspace::OptimizeLevel::O2 => "speed",
                destack_workspace::OptimizeLevel::O3 => "speed_and_size",
            }
        } else {
            "none"
        };
        flags_builder
            .set("opt_level", opt_level)
            .map_err(|e| CraneliftError::Internal {
                message: e.to_string(),
            })?;

        let flags = settings::Flags::new(flags_builder);

        let isa_builder = cranelift_codegen::isa::lookup(triple.clone()).map_err(|e| {
            CraneliftError::UnsupportedTarget {
                triple: e.to_string(),
            }
        })?;

        isa_builder
            .finish(flags)
            .map_err(|e| CraneliftError::Internal {
                message: e.to_string(),
            })
    }

    /// Get the target ISA.
    pub fn isa(&self) -> &dyn TargetIsa {
        self.isa.as_ref()
    }

    /// Compile a MIR module to bytes.
    ///
    /// For WASM targets, returns a `.wasm` module.
    /// For native targets, returns an object file.
    pub fn compile_module(
        &self,
        tree: &NodeTree,
        strings: &ImmutableStringPool,
    ) -> Result<Vec<u8>, CraneliftError> {
        let mut context = ModuleLowerer::new(self.isa.as_ref(), strings);
        context.lower_module(tree)?;
        context.finish()
    }

    /// Compile a MIR module and return Cranelift IR text format.
    ///
    /// Useful for debugging and testing.
    pub fn compile_to_clif(
        &self,
        tree: &NodeTree,
        strings: &ImmutableStringPool,
    ) -> Result<String, CraneliftError> {
        let mut context = ModuleLowerer::new(self.isa.as_ref(), strings);
        context.lower_module(tree)?;
        context.to_clif_string()
    }
}

impl CodegenBackend for CraneliftCodegenBackend {
    type Output = Vec<u8>;
    type Error = CraneliftError;

    fn compile_module(
        &self,
        tree: &NodeTree,
        strings: &ImmutableStringPool,
    ) -> Result<Self::Output, Self::Error> {
        CraneliftCodegenBackend::compile_module(self, tree, strings)
    }
}
