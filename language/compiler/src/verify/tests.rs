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
use mir::parse::ParseOptions;

use crate::verify::{DropInsert, MemoryCheck, VerifyError, VerifyState};

/// Placeholder module id for verify tests.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Placeholder profile id for verify tests.
fn test_profile_id() -> ProfileId {
    ProfileId::new(0)
}

/// Placeholder target id for verify tests.
fn test_target_id() -> TargetId {
    TargetId::new(PackageId::new(0), "test")
}

/// Provider context used by verify tests.
struct TestProvider;

impl DiagnosticContext for TestProvider {
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

    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Ok(format!("{display:?}"))
    }
}

impl ProviderContext for TestProvider {
    fn revision(&self) -> Revision {
        Revision::NULL
    }

    fn artifact_key(&self) -> ArtifactKey {
        ArtifactKey::mir_verified(test_module_id(), test_profile_id(), test_target_id())
    }

    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        Err(ProviderError::Blocked { keys: vec![key] })
    }

    fn require_all(&self, keys: &[ArtifactKey]) -> Result<Vec<ArtifactVersion>, ProviderError> {
        Err(ProviderError::Blocked {
            keys: keys.to_vec(),
        })
    }

    fn track(&self, _dependency: ArtifactDependency) {}

    fn emit_collection(&self, _diagnostics: DiagnosticCollection) {}

    fn emit(&self, _diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        Ok(())
    }
}

/// MIR program under verify tests.
pub(super) struct VerifyProgram {
    /// MIR tree.
    pub tree: mir::Tree,
    /// String pool.
    strings: StringPool,
    /// Test provider context.
    provider: TestProvider,
}

impl VerifyProgram {
    /// Parse one MIR program.
    pub(super) fn new(source: &str) -> Self {
        let (tree, strings) =
            mir::parse::Parser::parse(FileId::new(0), source, ParseOptions::default())
                .finish()
                .expect("failed to parse MIR");

        Self {
            tree,
            strings,
            provider: TestProvider,
        }
    }

    /// Run memory verification.
    pub(super) fn run_memory(&mut self) -> Vec<VerifyError> {
        let mut state = VerifyState::new(
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            &self.provider,
        );
        MemoryCheck.run(&mut self.tree, &mut state);

        collect_errors(&state)
    }

    /// Run drop insertion.
    pub(super) fn run_drop(&mut self) -> String {
        let mut state = VerifyState::new(
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            &self.provider,
        );
        DropInsert.run(&mut self.tree, &mut state);

        mir::format_mir(&self.tree, &self.strings, mir::MirFormatOptions::default())
    }
}

/// Collect typed verify errors.
fn collect_errors(state: &VerifyState<'_>) -> Vec<VerifyError> {
    state
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.diagnostic().clone())
        .collect()
}
