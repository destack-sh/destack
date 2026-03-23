use std::str::FromStr;
use std::sync::Arc;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use destack_artifact::{
    ArtifactStore, BinaryArtifact, EmitFormat, ObjectArtifact, WasmArtifact, WasmInterface,
};
use destack_codegen_lib::CodegenBackend;
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::{Program, Target, TargetId};
use target_lexicon::Triple;

use crate::lower::{ModuleLowerOutput, ModuleLowerer};
use crate::{CodegenCraneliftError, CodegenCraneliftResult, CodegenCraneliftWarning};

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
            debug: target.debug,
        })
    }

    /// Create a backend for WebAssembly output.
    pub fn wasm() -> CodegenCraneliftResult<Self> {
        Self::new(&Target::wasm_js("wasm"))
    }

    /// Create a backend for native output (host triple).
    pub fn native() -> CodegenCraneliftResult<Self> {
        Self::new(&Target::native("native"))
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
        let opt_level = if target.optimize {
            match target.optimize_level {
                destack_workspace::OptimizeLevel::O0 => "none",
                destack_workspace::OptimizeLevel::O1 => "speed",
                destack_workspace::OptimizeLevel::O2 => "speed",
                destack_workspace::OptimizeLevel::O3 => "speed_and_size",
                destack_workspace::OptimizeLevel::O4 => "speed_and_size",
            }
        } else {
            "none"
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
        tree: &mir::NodeTree,
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
        tree: &mir::NodeTree,
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

/// Generate one binary artifact for a module using Cranelift.
pub fn generate_artifact(
    program: Arc<Program>,
    artifacts: Arc<ArtifactStore>,
    module_id: ModuleId,
    target: &Target,
) -> CodegenCraneliftResult<(
    BinaryArtifact,
    Vec<CodegenCraneliftWarning>,
    Vec<CodegenCraneliftError>,
)> {
    // validate target
    match target.emit {
        EmitFormat::Wasm | EmitFormat::Native => {}
        other => {
            return Err(CodegenCraneliftError::UnsupportedTarget {
                triple: format!("{other:?}"),
                message: Some("expected Wasm or Native".to_string()),
            });
        }
    }

    // create backend
    let backend = CodegenCraneliftBackend::new(target)?;

    // get module and its MIR
    // compile
    let module_ref = program.modules.get(module_id);
    let module = module_ref.as_ref();
    let name = module.uri.last_segment().unwrap_or("module");
    let profile_id = program.default_profile_id_for_module(module_id);
    let target_id = TargetId::new(module.package_id, target.name.clone());
    let compile_output =
        if let Some(mir) = artifacts.mir_optimized(module_id, profile_id, &target_id) {
            backend.compile_module(&mir.tree, &mir.strings, name)?
        } else if let Some(mir) = artifacts.mir_base(module_id, profile_id, &target_id) {
            backend.compile_module(&mir.tree, &mir.strings, name)?
        } else {
            panic!("codegen requires committed MIR artifact");
        };

    let artifact = match target.emit {
        EmitFormat::Native => BinaryArtifact::Object(ObjectArtifact {
            bytes: Arc::from(compile_output.bytes),
            debug: Vec::new(),
        }),
        EmitFormat::Wasm => BinaryArtifact::Wasm(WasmArtifact {
            bytes: Arc::from(compile_output.bytes),
            interface: WasmInterface::default(),
            source_map: None,
        }),
        _ => unreachable!(),
    };

    Ok((artifact, compile_output.warnings, compile_output.errors))
}
