use destack_artifact::{
    ArtifactDependency, ArtifactKey, ArtifactVersion, DiagnosticAnchor, DiagnosticContext,
    DiagnosticDisplay, DiagnosticError, DiagnosticLike,
};
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{
    DiagnosticCollection, DiagnosticLabel, FileContentId, FileId, ModuleId, PackageId, ProfileId,
    Span, TargetId,
};
use destack_workspace::{ProviderContext, ProviderError, Revision};

/// MIR program under compiler tests.
pub(crate) struct TestProgram {
    /// MIR tree.
    pub(crate) tree: mir::Tree,
    /// String pool.
    pub(crate) strings: StringPool,
    /// Test provider context.
    pub(crate) provider: TestMirProvider,
}

impl TestProgram {
    /// Parse one MIR program.
    pub(crate) fn mir(source: &str) -> Self {
        let (tree, strings) =
            mir::parse::Parser::parse(FileId::new(0), source, mir::parse::ParseOptions::default())
                .finish()
                .expect("failed to parse MIR");

        Self {
            tree,
            strings,
            provider: TestMirProvider,
        }
    }

    /// Return the test module id.
    pub(crate) fn module_id(&self) -> ModuleId {
        test_module_id()
    }

    /// Return the test profile id.
    pub(crate) fn profile_id(&self) -> ProfileId {
        test_profile_id()
    }

    /// Return the test target id.
    pub(crate) fn target_id(&self) -> TargetId {
        test_target_id()
    }
}

/// Provider context used by raw MIR tests.
pub(crate) struct TestMirProvider;

impl DiagnosticContext for TestMirProvider {
    /// Resolve one diagnostic anchor into a source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let span = match anchor {
            DiagnosticAnchor::Span(span) => *span,
            DiagnosticAnchor::File(_)
            | DiagnosticAnchor::Module(_)
            | DiagnosticAnchor::Package(_) => Span::empty(FileId::new(0)),
        };

        Ok(DiagnosticLabel {
            content: FileContentId::new(0),
            span,
            message,
        })
    }

    /// Display one diagnostic value.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Ok(format!("{display:?}"))
    }
}

impl ProviderContext for TestMirProvider {
    /// Return the null test revision.
    fn revision(&self) -> Revision {
        Revision::NULL
    }

    /// Return the raw MIR verified artifact key.
    fn artifact_key(&self) -> ArtifactKey {
        ArtifactKey::mir_verified(test_module_id(), test_profile_id(), test_target_id())
    }

    /// Reject artifact requirements in raw MIR tests.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        Err(ProviderError::Blocked { keys: vec![key] })
    }

    /// Reject artifact requirements in raw MIR tests.
    fn require_all(&self, keys: &[ArtifactKey]) -> Result<Vec<ArtifactVersion>, ProviderError> {
        Err(ProviderError::Blocked {
            keys: keys.to_vec(),
        })
    }

    /// Track one dependency.
    fn track(&self, _dependency: ArtifactDependency) {}

    /// Emit one diagnostic collection.
    fn emit_collection(&self, _diagnostics: DiagnosticCollection) {}

    /// Emit one diagnostic.
    fn emit(&self, _diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        Ok(())
    }
}

/// Return the raw MIR test module id.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Return the raw MIR test profile id.
fn test_profile_id() -> ProfileId {
    ProfileId::new(0)
}

/// Return the raw MIR test target id.
fn test_target_id() -> TargetId {
    TargetId::new(PackageId::new(0), "test")
}
