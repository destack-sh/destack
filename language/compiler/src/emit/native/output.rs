use crate::{Compiler, CompilerError, CompilerResult, DiagnosticAnchor, EmitError};
use destack_artifact::{ModuleOutput, NativeOutput};
use destack_codegen_native::CodegenCraneliftError;
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, Target};
use destack_source::{Content, ModuleId, TargetId};

impl Compiler {
    /// Emit one native module output through the native backend.
    pub(in crate::emit) fn emit_native_module_output(
        &self,
        module_id: ModuleId,
        target: &Target,
        target_id: &TargetId,
        profile: ProfileId,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<ModuleOutput> {
        // load the owning module once for backend context
        let module = self.module(context.revision(), module_id)?;

        // load the MIR state
        let mir_optimized = artifacts
            .mir_optimized(module_id, profile, *target_id)
            .map_err(CompilerError::from)?;
        let mir_lowered = artifacts
            .mir_lowered(module_id, profile, *target_id)
            .map_err(CompilerError::from)?;
        let state = NativeEmitState::new(
            module_id,
            mir_optimized
                .latest_patch_tree()
                .unwrap_or(&mir_lowered.tree),
        );

        // emit one native output through the current backend
        let (output, errors) = destack_codegen_native::NativeOutputGenerator::new(
            module.clone(),
            self.repository.string_pool().clone(),
            Some(mir_optimized.clone()),
            Some(mir_lowered.clone()),
            target,
        )
        .generate()
        .map_err(|error| Self::map_native_emit_error(&state, error))?;

        // map backend diagnostics into compiler diagnostics
        for error in errors {
            let error = Self::map_native_emit_error(&state, error);
            self.emit_diagnostic(context, error)?;
        }

        // intern emitted native bytes in repository contents
        let content = self.repository.intern_content(Content::Binary {
            content: output.bytes,
        })?;
        let artifact = NativeOutput::new(output.file_type, content, output.source_map);

        Ok(ModuleOutput::Native(Box::new(artifact)))
    }

    /// Map one native backend error to a compiler error.
    fn map_native_emit_error(
        state: &NativeEmitState<'_>,
        error: CodegenCraneliftError,
    ) -> EmitError {
        match error {
            CodegenCraneliftError::UnsupportedTarget { triple, .. } => {
                EmitError::UnsupportedTarget {
                    anchor: (state.module_id).into(),
                    module: state.module_id,
                    target: triple,
                }
            }
            CodegenCraneliftError::FunctionNotFound { name, .. } => EmitError::UnresolvedFunction {
                anchor: (state.module_id).into(),
                module: state.module_id,
                name,
            },
            CodegenCraneliftError::Internal { message } => EmitError::Internal {
                anchor: (state.module_id).into(),
                module: state.module_id,
                message,
            },
            CodegenCraneliftError::UnsupportedType { node, .. } => EmitError::UnsupportedType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenCraneliftError::MissingType { node, .. } => EmitError::MissingType {
                anchor: state.anchor(node),
                module: state.module_id,
            },
            CodegenCraneliftError::UnsupportedInstruction { node, .. } => {
                EmitError::UnsupportedConstruct {
                    anchor: state.anchor(node),
                    module: state.module_id,
                    message: "unsupported instruction".to_string(),
                }
            }
            CodegenCraneliftError::OutOfBounds { node, index, len } => EmitError::OutOfBounds {
                anchor: state.anchor(node),
                module: state.module_id,
                index,
                len,
            },
        }
    }
}

/// State for mapping native backend diagnostics into compiler diagnostics.
struct NativeEmitState<'a> {
    /// The module being emitted.
    module_id: ModuleId,
    /// The source-bearing MIR tree used by the backend.
    tree: &'a mir::Tree,
}

impl<'a> NativeEmitState<'a> {
    /// Create native emit diagnostic mapping state.
    fn new(module_id: ModuleId, tree: &'a mir::Tree) -> Self {
        Self { module_id, tree }
    }

    /// Return the source anchor for one backend MIR node.
    fn anchor(&self, node: mir::LocalNodeIdAny) -> DiagnosticAnchor {
        let span = self
            .tree
            .get_span_by_id(node.id)
            .expect("native diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
    }
}
