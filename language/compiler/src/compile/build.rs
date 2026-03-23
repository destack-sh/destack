use destack_artifact::{ArtifactDependency, ArtifactKey};

use crate::{DiagnosticAnchor, TaskError};

/// Requirement for one artifact key.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRequirement {
    /// The diagnostic anchor for this requirement.
    pub anchor: DiagnosticAnchor,
    /// The required artifact key.
    pub key: ArtifactKey,
    /// The expected dependency for that key.
    pub dependency: ArtifactDependency,
    /// Optional fallback error if the requirement cannot be satisfied.
    pub error: Option<Box<TaskError>>,
}

impl ArtifactRequirement {
    /// Create a new artifact requirement.
    pub fn new(anchor: DiagnosticAnchor, key: ArtifactKey, dependency: ArtifactDependency) -> Self {
        Self {
            anchor,
            key,
            dependency,
            error: None,
        }
    }

    /// Attach a fallback error to this requirement.
    pub fn with_error(self, error: TaskError) -> Self {
        Self {
            error: Some(Box::new(error)),
            ..self
        }
    }
}

/// Requirement algebra for the unified artifact graph.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactRequirementSet {
    /// Require one artifact key.
    One(ArtifactRequirement),
    /// Require all listed artifact keys.
    All(Vec<ArtifactRequirement>),
}

impl ArtifactRequirementSet {
    /// Create a requirement set for one artifact key.
    pub fn one(requirement: ArtifactRequirement) -> Self {
        Self::One(requirement)
    }

    /// Return the first concrete requirement for diagnostics and tracing.
    pub fn first(&self) -> Option<&ArtifactRequirement> {
        match self {
            Self::One(requirement) => Some(requirement),
            Self::All(requirements) => requirements.first(),
        }
    }

    /// Return the diagnostic anchor for this requirement set.
    pub fn anchor(&self) -> DiagnosticAnchor {
        self.first()
            .map(|requirement| requirement.anchor.clone())
            .unwrap_or(DiagnosticAnchor::Global)
    }

    /// Return the number of concrete requirements in this set.
    pub fn len(&self) -> usize {
        match self {
            Self::One(..) => 1,
            Self::All(requirements) => requirements.len(),
        }
    }

    /// Return whether this set has no concrete requirements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Visit each concrete requirement in this set.
    pub fn for_each(&self, mut handle: impl FnMut(&ArtifactRequirement)) {
        match self {
            Self::One(requirement) => handle(requirement),
            Self::All(requirements) => {
                for requirement in requirements {
                    handle(requirement);
                }
            }
        }
    }

    /// Return true when any concrete requirement matches the predicate.
    pub fn any(&self, mut predicate: impl FnMut(&ArtifactRequirement) -> bool) -> bool {
        match self {
            Self::One(requirement) => predicate(requirement),
            Self::All(requirements) => requirements.iter().any(predicate),
        }
    }

    /// Return true when all concrete requirements match the predicate.
    pub fn all(&self, mut predicate: impl FnMut(&ArtifactRequirement) -> bool) -> bool {
        match self {
            Self::One(requirement) => predicate(requirement),
            Self::All(requirements) => requirements.iter().all(predicate),
        }
    }

    /// Return the first mapped value from the concrete requirements.
    pub fn find_map<T>(
        &self,
        mut handle: impl FnMut(&ArtifactRequirement) -> Option<T>,
    ) -> Option<T> {
        match self {
            Self::One(requirement) => handle(requirement),
            Self::All(requirements) => requirements.iter().find_map(handle),
        }
    }

    /// Convert this requirement set into a flat vector.
    pub fn into_requirements(self) -> Vec<ArtifactRequirement> {
        match self {
            Self::One(requirement) => vec![requirement],
            Self::All(requirements) => requirements,
        }
    }
}

/// Error when an artifact requirement is not satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactRequirementError {
    /// Artifact is not yet available, need to yield.
    NotReady { requirement: ArtifactRequirementSet },
    /// Upstream artifact build has failed.
    Failed { requirement: ArtifactRequirementSet },
}

impl ArtifactRequirementError {
    /// Return the carried requirement.
    pub fn requirement(&self) -> &ArtifactRequirementSet {
        match self {
            Self::NotReady { requirement } | Self::Failed { requirement } => requirement,
        }
    }

    /// Convert into the carried requirement.
    pub fn into_requirement(self) -> ArtifactRequirementSet {
        match self {
            Self::NotReady { requirement } | Self::Failed { requirement } => requirement,
        }
    }
}

impl TryFrom<ArtifactRequirementError> for ArtifactRequirementSet {
    type Error = ArtifactRequirementError;

    fn try_from(error: ArtifactRequirementError) -> Result<Self, Self::Error> {
        match error {
            ArtifactRequirementError::NotReady { requirement } => Ok(requirement),
            ArtifactRequirementError::Failed { .. } => Err(error),
        }
    }
}

/// Collector for coalescing artifact requirements from multiple operations.
#[derive(Debug, Default)]
pub struct ArtifactRequirementCollector {
    requirements: Vec<ArtifactRequirement>,
}

impl ArtifactRequirementCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self {
            requirements: Vec::new(),
        }
    }

    /// Collect a result as an artifact requirement.
    pub fn try_collect<T, E>(&mut self, result: Result<T, E>) -> Option<E>
    where
        E: TryInto<ArtifactRequirementSet, Error = E>,
    {
        match result {
            Ok(_) => None,
            Err(error) => match error.try_into() {
                Ok(requirement) => {
                    self.requirements.extend(requirement.into_requirements());
                    None
                }
                Err(error) => Some(error),
            },
        }
    }

    /// Return whether any requirements were collected.
    pub fn has_requirements(&self) -> bool {
        !self.requirements.is_empty()
    }

    /// Finish collection and require all collected requirements.
    pub fn try_into_requirement(self) -> Option<ArtifactRequirementSet> {
        match self.requirements.len() {
            0 => None,
            1 => self
                .requirements
                .into_iter()
                .next()
                .map(ArtifactRequirementSet::One),
            _ => Some(ArtifactRequirementSet::All(self.requirements)),
        }
    }
}
