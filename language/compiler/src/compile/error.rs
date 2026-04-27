use destack_artifact::{ArtifactKey, ArtifactStore};
use destack_workspace::{Repository, Revision};

use crate::emit::EmitError;
use crate::{
    AnalyzeError, CompilePhase, DiagnosticAnchor, ElaborateError, ExecuteError, GenerateError,
    ImportError, LinkError, LowerError, OptimizeError, RequirementError, RequirementSet,
    ResolveError,
};
/// Error during compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    /// Wait for artifact requirements.
    Yield { requirement: RequirementSet },
    /// Artifact requirement failed upstream.
    UnsatisfiedRequirement { requirement: RequirementSet },
    /// Error during importing.
    Import(ImportError),
    /// Error during resolution.
    Resolve(ResolveError),
    /// Error during analysis.
    Analyze(AnalyzeError),
    /// Error during elaboration.
    Elaborate(ElaborateError),
    // --------------------------------------------------
    /// Error during execution.
    Execute(ExecuteError),
    /// Error during lowering.
    Lower(LowerError),
    /// Error during optimization.
    Optimize(OptimizeError),
    // --------------------------------------------------
    /// Error during generating.
    Generate(GenerateError),
    /// Error during linking.
    Link(LinkError),
    /// Error during emitting.
    Emit(EmitError),
    /// Internal compiler error (bug).
    Internal(InternalError),
}

/// Internal compiler error (bug in the compiler).
#[derive(Debug, Clone, PartialEq)]
pub enum InternalError {
    /// One compiler entrypoint received a revision that is not semantically usable.
    InvalidRevision { revision: Revision, message: String },
    /// Task yielded to the same requirement twice in a row.
    SuspiciousYield { requirement: RequirementSet },
    /// Task exceeded maximum yield count.
    ExcessiveYield {
        artifact_key: ArtifactKey,
        requirement: RequirementSet,
        yield_count: u32,
    },
    /// Circular artifact dependency detected in the scheduler.
    CircularDependency { cycle: Vec<ArtifactKey> },
}

impl InternalError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u16 {
        match self {
            Self::InvalidRevision { .. } => 1,
            Self::SuspiciousYield { .. } => 2,
            Self::ExcessiveYield { .. } => 3,
            Self::CircularDependency { .. } => 4,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        // internal errors are global, not tied to specific source
        DiagnosticAnchor::Global
    }

    /// Get the message of the error.
    pub fn message(&self, _repository: &Repository, _artifacts: &ArtifactStore) -> String {
        match self {
            Self::InvalidRevision { revision, message } => {
                format!("internal error: invalid compiler revision {revision}: {message}")
            }
            Self::SuspiciousYield { .. } => {
                "internal error: artifact provide yielded to the same requirement twice".to_string()
            }
            Self::ExcessiveYield {
                artifact_key,
                requirement,
                yield_count,
                ..
            } => {
                let requirement_description = if let Some(first_requirement) = requirement.first() {
                    if requirement.len() == 1 {
                        match first_requirement {
                            crate::Requirement::Artifact(requirement) => {
                                format!("{:?}", requirement.version.key)
                            }
                            crate::Requirement::File(requirement) => {
                                format!("{:?}", requirement.change)
                            }
                        }
                    } else {
                        format!("all({} requirements)", requirement.len())
                    }
                } else {
                    "no requirements".to_string()
                };

                format!(
                    "internal error: artifact {} yielded {yield_count} times on {requirement_description}",
                    artifact_key.name(),
                )
            }

            Self::CircularDependency { cycle, .. } => {
                let cycle_str = cycle
                    .iter()
                    .map(|artifact_key| format!("{artifact_key:?}"))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                format!("internal error: circular artifact requirement: {cycle_str}")
            }
        }
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EZ{:03}", self.sub_code())
    }
}

impl From<InternalError> for CompileError {
    #[inline]
    fn from(error: InternalError) -> Self {
        CompileError::Internal(error)
    }
}

impl From<RequirementError> for CompileError {
    fn from(error: RequirementError) -> Self {
        match error {
            RequirementError::NotReady { requirement } => Self::Yield { requirement },
            RequirementError::Failed { requirement } => {
                Self::UnsatisfiedRequirement { requirement }
            }
        }
    }
}

impl CompileError {
    /// Get the phase of the error, if applicable.
    pub fn phase(&self) -> Option<CompilePhase> {
        match self {
            Self::Yield { .. } | Self::UnsatisfiedRequirement { .. } => None,
            Self::Import(_) => Some(CompilePhase::Import),
            Self::Resolve(_) => Some(CompilePhase::Resolve),
            Self::Analyze(_) => Some(CompilePhase::Analyze),
            Self::Elaborate(_) => Some(CompilePhase::Elaborate),
            Self::Execute(_) => Some(CompilePhase::Execute),
            Self::Lower(_) => Some(CompilePhase::Lower),
            Self::Optimize(_) => Some(CompilePhase::Optimize),
            Self::Generate(_) => Some(CompilePhase::Generate),
            Self::Link(_) => Some(CompilePhase::Link),
            Self::Emit(_) => None,
            Self::Internal(_) => None,
        }
    }

    /// Get the phase letter of the error.
    pub fn phase_letter(&self) -> char {
        match self.phase() {
            Some(phase) => phase.letter(),
            None => 'Z', // Z for internal compiler error
        }
    }

    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u16 {
        match self {
            Self::Yield { .. } | Self::UnsatisfiedRequirement { .. } => 0,
            Self::Import(error) => error.sub_code(),
            Self::Resolve(error) => error.sub_code(),
            Self::Analyze(error) => error.sub_code(),
            Self::Elaborate(error) => error.sub_code(),
            Self::Execute(error) => error.sub_code(),
            Self::Lower(error) => error.sub_code(),
            Self::Optimize(error) => error.sub_code(),
            Self::Generate(error) => error.sub_code(),
            Self::Link(error) => error.sub_code(),
            Self::Emit(error) => error.sub_code(),
            Self::Internal(error) => error.sub_code(),
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Yield { .. } | Self::UnsatisfiedRequirement { .. } => DiagnosticAnchor::Global,
            Self::Import(error) => error.anchor(),
            Self::Resolve(error) => error.anchor(),
            Self::Analyze(error) => error.anchor(),
            Self::Elaborate(error) => error.anchor(),
            Self::Execute(error) => error.anchor(),
            Self::Lower(error) => error.anchor(),
            Self::Optimize(error) => error.anchor(),
            Self::Generate(error) => error.anchor(),
            Self::Link(error) => error.anchor(),
            Self::Emit(error) => error.anchor(),
            Self::Internal(error) => error.anchor(),
        }
    }

    /// Get the message of the error.
    pub fn message(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        match self {
            Self::Yield { .. } => {
                "internal error: compiler requirement yield escaped diagnostics".to_string()
            }
            Self::UnsatisfiedRequirement { .. } => {
                "internal error: compiler requirement failed upstream".to_string()
            }
            Self::Import(error) => error.message(revision, repository, artifacts),
            Self::Resolve(error) => error.message(revision, repository, artifacts),
            Self::Analyze(error) => error.message(revision, repository, artifacts),
            Self::Elaborate(error) => error.message(revision, repository, artifacts),
            Self::Execute(error) => error.message(revision, repository, artifacts),
            Self::Lower(error) => error.message(revision, repository, artifacts),
            Self::Optimize(error) => error.message(revision, repository, artifacts),
            Self::Generate(error) => error.message(revision, repository, artifacts),
            Self::Link(error) => error.message(revision, repository, artifacts),
            Self::Emit(error) => error.message(revision, repository, artifacts),
            Self::Internal(error) => error.message(repository, artifacts),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("E{}{:03}", self.phase_letter(), self.sub_code())
    }

    /// Check whether this error represents a failed requirement yield.
    pub fn is_yield_failed(&self) -> bool {
        match self {
            Self::Yield { .. } => false,
            Self::UnsatisfiedRequirement { .. } => true,
            Self::Import(error) => error.is_yield_failed(),
            Self::Resolve(error) => error.is_yield_failed(),
            Self::Analyze(error) => error.is_yield_failed(),
            Self::Elaborate(error) => error.is_yield_failed(),
            Self::Execute(error) => error.is_yield_failed(),
            Self::Lower(error) => error.is_yield_failed(),
            Self::Optimize(error) => error.is_yield_failed(),
            Self::Generate(error) => error.is_yield_failed(),
            Self::Link(error) => error.is_yield_failed(),
            Self::Emit(error) => error.is_yield_failed(),
            Self::Internal(_) => false,
        }
    }

    /// Get the yielded artifact requirement, if any.
    pub fn yielded_to(&self) -> Option<&RequirementSet> {
        match self {
            Self::Yield { requirement } => Some(requirement),
            Self::Import(ImportError::Yield { requirement }) => Some(requirement),
            Self::Resolve(ResolveError::Yield { requirement }) => Some(requirement),
            Self::Analyze(AnalyzeError::Yield { requirement }) => Some(requirement),
            Self::Elaborate(ElaborateError::Yield { requirement }) => Some(requirement),
            Self::Execute(ExecuteError::Yield { requirement }) => Some(requirement),
            Self::Lower(LowerError::Yield { requirement }) => Some(requirement),
            Self::Optimize(OptimizeError::Yield { requirement }) => Some(requirement),
            Self::Generate(GenerateError::Yield { requirement }) => Some(requirement),
            Self::Link(LinkError::Yield { requirement }) => Some(requirement),
            Self::Emit(EmitError::Yield { requirement }) => Some(requirement),
            _ => None,
        }
    }

    /// Return the blocking artifact requirement for this error, if any.
    pub fn blocking_requirement(&self) -> Option<&RequirementSet> {
        match self {
            Self::Yield { requirement } | Self::UnsatisfiedRequirement { requirement } => {
                Some(requirement)
            }
            Self::Import(ImportError::Yield { requirement })
            | Self::Import(ImportError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Import(_) => None,
            Self::Resolve(ResolveError::Yield { requirement })
            | Self::Resolve(ResolveError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Resolve(_) => None,
            Self::Analyze(AnalyzeError::Yield { requirement })
            | Self::Analyze(AnalyzeError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Analyze(_) => None,
            Self::Elaborate(ElaborateError::Yield { requirement })
            | Self::Elaborate(ElaborateError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Elaborate(_) => None,
            Self::Execute(ExecuteError::Yield { requirement })
            | Self::Execute(ExecuteError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Execute(_) => None,
            Self::Lower(LowerError::Yield { requirement })
            | Self::Lower(LowerError::UnsatisfiedRequirement { requirement }) => Some(requirement),
            Self::Lower(_) => None,
            Self::Optimize(OptimizeError::Yield { requirement })
            | Self::Optimize(OptimizeError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Optimize(_) => None,
            Self::Generate(GenerateError::Yield { requirement })
            | Self::Generate(GenerateError::UnsatisfiedRequirement { requirement }) => {
                Some(requirement)
            }
            Self::Generate(_) => None,
            Self::Link(LinkError::Yield { requirement })
            | Self::Link(LinkError::UnsatisfiedRequirement { requirement }) => Some(requirement),
            Self::Link(_) => None,
            Self::Emit(EmitError::Yield { requirement })
            | Self::Emit(EmitError::UnsatisfiedRequirement { requirement }) => Some(requirement),
            Self::Emit(_) => None,
            Self::Internal(_) => None,
        }
    }
}
