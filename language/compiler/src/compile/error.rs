use destack_artifact::{
    ArtifactFailure, ArtifactKey, ArtifactVersion, DiagnosticError, DiagnosticLike,
};
use destack_repository::ProviderError;

use crate::{
    BindError, CheckError, ElaborateError, ExpandError, ExportError, GenerateError, ImportError,
    LinkError, LowerError, MaterializeError, OptimizeError, ResolveError, VerifyError,
};

/// Compiler-local error while providing one artifact.
#[derive(Debug)]
pub(crate) enum CompilerError {
    /// The compiler needs more source or artifacts first.
    Blocked { keys: Vec<ArtifactKey> },
    /// One required artifact failed upstream.
    RequirementFailed { key: ArtifactKey },
    /// The artifact store has a ready version without the expected payload.
    Corrupt { version: ArtifactVersion },
    /// The compiler produced diagnostics without a payload.
    Diagnostic(Box<dyn DiagnosticLike>),
    /// The compiler failed due to an infrastructure error.
    Internal { message: String },
}

/// Result type for compiler-local provider helpers.
pub(crate) type CompilerResult<T> = Result<T, CompilerError>;

impl From<ProviderError> for CompilerError {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::Blocked { keys } => Self::Blocked { keys },
            ProviderError::RequirementFailed { key } => Self::RequirementFailed { key },
            ProviderError::Corrupt { version } => Self::Corrupt { version },
            ProviderError::Failed { failure } => Self::Internal {
                message: format!("provider failed without compiler diagnostic: {failure:?}"),
            },
            ProviderError::Internal { message } => Self::Internal { message },
        }
    }
}

impl From<DiagnosticError> for CompilerError {
    /// Convert diagnostic finalization failure into provider boundary failure.
    fn from(error: DiagnosticError) -> Self {
        Self::Internal {
            message: format!("failed to finalize compiler diagnostic: {error}"),
        }
    }
}

macro_rules! impl_compiler_error_from_diagnostic {
    ($diagnostic:ty) => {
        impl From<$diagnostic> for CompilerError {
            fn from(diagnostic: $diagnostic) -> Self {
                Self::Diagnostic(Box::new(diagnostic))
            }
        }
    };
}

impl_compiler_error_from_diagnostic!(CheckError);
impl_compiler_error_from_diagnostic!(BindError);
impl_compiler_error_from_diagnostic!(ElaborateError);
impl_compiler_error_from_diagnostic!(MaterializeError);
impl_compiler_error_from_diagnostic!(ExpandError);
impl_compiler_error_from_diagnostic!(ExportError);
impl_compiler_error_from_diagnostic!(GenerateError);
impl_compiler_error_from_diagnostic!(ImportError);
impl_compiler_error_from_diagnostic!(LinkError);
impl_compiler_error_from_diagnostic!(LowerError);
impl_compiler_error_from_diagnostic!(OptimizeError);
impl_compiler_error_from_diagnostic!(ResolveError);
impl_compiler_error_from_diagnostic!(VerifyError);

impl CompilerError {
    /// Convert provider boundary control flow into a provider error.
    pub(crate) fn into_provider_error(self) -> ProviderError {
        match self {
            Self::Blocked { keys } => ProviderError::Blocked { keys },
            Self::RequirementFailed { key } => ProviderError::RequirementFailed { key },
            Self::Corrupt { version } => ProviderError::Corrupt { version },
            Self::Diagnostic(_) => ProviderError::failed(ArtifactFailure::diagnostics()),
            Self::Internal { message } => ProviderError::Internal { message },
        }
    }
}
