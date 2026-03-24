use std::sync::Arc;

use destack_artifact::{
    ArtifactStore, BinaryArtifact, EmitFormat, ObjectArtifact, WasmArtifact, WasmInterface,
};
use destack_source::ModuleId;
use destack_workspace::{Program, Target, TargetId};

use crate::{CodegenCraneliftError, CodegenCraneliftResult, CodegenCraneliftWarning};

/// One generator for binary module artifacts.
#[derive(Debug)]
pub struct BinaryArtifactGenerator<'a> {
    /// The shared program state.
    program: Arc<Program>,
    /// The shared artifact store.
    artifacts: Arc<ArtifactStore>,
    /// The module to generate.
    module_id: ModuleId,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> BinaryArtifactGenerator<'a> {
    /// Create one binary artifact generator.
    pub fn new(
        program: Arc<Program>,
        artifacts: Arc<ArtifactStore>,
        module_id: ModuleId,
        target: &'a Target,
    ) -> Self {
        Self {
            program,
            artifacts,
            module_id,
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
        let module_ref = self.program.modules.get(self.module_id);
        let module = module_ref.as_ref();
        let name = module.uri.last_segment().unwrap_or("module");
        let profile_id = self.program.default_profile_id_for_module(self.module_id);
        let target_id = TargetId::new(module.package_id, self.target.name.clone());
        let compile_output = if let Some(mir) =
            self.artifacts
                .mir_optimized(self.module_id, profile_id, &target_id)
        {
            backend.compile_module(&mir.tree, &mir.strings, name)?
        } else if let Some(mir) = self
            .artifacts
            .mir_base(self.module_id, profile_id, &target_id)
        {
            backend.compile_module(&mir.tree, &mir.strings, name)?
        } else {
            panic!("codegen requires committed MIR artifact");
        };

        let artifact = match self.target.emit {
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
}
