use destack_workspace::{ArtifactDependency, ArtifactKey, OutputDependency, OutputKey};

use crate::{DiagnosticAnchor, TaskError};

/// Unified build graph key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuildKey {
    /// Semantic compiler product.
    Artifact(ArtifactKey),
    /// Build product.
    Output(OutputKey),
}

impl BuildKey {
    /// Create one build key for an artifact.
    pub fn artifact(key: ArtifactKey) -> Self {
        Self::Artifact(key)
    }

    /// Create one build key for an output.
    pub fn output(key: OutputKey) -> Self {
        Self::Output(key)
    }
}

/// Dependency stamp for one build key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildDependency {
    /// Dependency for a semantic compiler product.
    Artifact(ArtifactDependency),
    /// Dependency for a build product.
    Output(OutputDependency),
}

/// Requirement for one build key.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildRequirement {
    /// The diagnostic anchor for this requirement.
    pub anchor: DiagnosticAnchor,
    /// The required build key.
    pub key: BuildKey,
    /// The expected dependency for that key.
    pub dependency: BuildDependency,
    /// Optional fallback error if the requirement cannot be satisfied.
    pub error: Option<Box<TaskError>>,
}

impl BuildRequirement {
    /// Create a new build requirement.
    pub fn new(anchor: DiagnosticAnchor, key: BuildKey, dependency: BuildDependency) -> Self {
        Self {
            anchor,
            key,
            dependency,
            error: None,
        }
    }

    /// Create one artifact build requirement.
    pub fn artifact(
        anchor: DiagnosticAnchor,
        key: ArtifactKey,
        dependency: ArtifactDependency,
    ) -> Self {
        Self::new(
            anchor,
            BuildKey::artifact(key),
            BuildDependency::Artifact(dependency),
        )
    }

    /// Create one output build requirement.
    pub fn output(anchor: DiagnosticAnchor, key: OutputKey, dependency: OutputDependency) -> Self {
        Self::new(
            anchor,
            BuildKey::output(key),
            BuildDependency::Output(dependency),
        )
    }

    /// Attach a fallback error to this requirement.
    pub fn with_error(self, error: TaskError) -> Self {
        Self {
            error: Some(Box::new(error)),
            ..self
        }
    }
}

/// Requirement algebra for the unified build graph.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildRequirementSet {
    /// Require one build key.
    One(BuildRequirement),
    /// Require all listed build keys.
    All(Vec<BuildRequirement>),
}

impl BuildRequirementSet {
    /// Create a requirement set for one build key.
    pub fn one(requirement: BuildRequirement) -> Self {
        Self::One(requirement)
    }

    /// Return the first concrete requirement for diagnostics and tracing.
    pub fn first(&self) -> Option<&BuildRequirement> {
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
    pub fn for_each(&self, mut handle: impl FnMut(&BuildRequirement)) {
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
    pub fn any(&self, mut predicate: impl FnMut(&BuildRequirement) -> bool) -> bool {
        match self {
            Self::One(requirement) => predicate(requirement),
            Self::All(requirements) => requirements.iter().any(predicate),
        }
    }

    /// Return true when all concrete requirements match the predicate.
    pub fn all(&self, mut predicate: impl FnMut(&BuildRequirement) -> bool) -> bool {
        match self {
            Self::One(requirement) => predicate(requirement),
            Self::All(requirements) => requirements.iter().all(predicate),
        }
    }

    /// Return the first mapped value from the concrete requirements.
    pub fn find_map<T>(&self, mut handle: impl FnMut(&BuildRequirement) -> Option<T>) -> Option<T> {
        match self {
            Self::One(requirement) => handle(requirement),
            Self::All(requirements) => requirements.iter().find_map(handle),
        }
    }

    /// Convert this requirement set into a flat vector.
    pub fn into_requirements(self) -> Vec<BuildRequirement> {
        match self {
            Self::One(requirement) => vec![requirement],
            Self::All(requirements) => requirements,
        }
    }
}

/// Error when a build requirement is not satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildRequirementError {
    /// Build is not yet complete, need to yield.
    NotReady { requirement: BuildRequirementSet },
    /// Build has failed.
    Failed { requirement: BuildRequirementSet },
}

impl BuildRequirementError {
    /// Return the carried requirement.
    pub fn requirement(&self) -> &BuildRequirementSet {
        match self {
            Self::NotReady { requirement } | Self::Failed { requirement } => requirement,
        }
    }

    /// Convert into the carried requirement.
    pub fn into_requirement(self) -> BuildRequirementSet {
        match self {
            Self::NotReady { requirement } | Self::Failed { requirement } => requirement,
        }
    }
}

impl TryFrom<BuildRequirementError> for BuildRequirementSet {
    type Error = BuildRequirementError;

    fn try_from(error: BuildRequirementError) -> Result<Self, Self::Error> {
        match error {
            BuildRequirementError::NotReady { requirement } => Ok(requirement),
            BuildRequirementError::Failed { .. } => Err(error),
        }
    }
}

/// Collector for coalescing build requirements from multiple operations.
#[derive(Debug, Default)]
pub struct BuildRequirementCollector {
    requirements: Vec<BuildRequirement>,
}

impl BuildRequirementCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self {
            requirements: Vec::new(),
        }
    }

    /// Collect a result as a build requirement.
    pub fn try_collect<T, E>(&mut self, result: Result<T, E>) -> Option<E>
    where
        E: TryInto<BuildRequirementSet, Error = E>,
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
    pub fn try_into_requirement(self) -> Option<BuildRequirementSet> {
        match self.requirements.len() {
            0 => None,
            1 => self
                .requirements
                .into_iter()
                .next()
                .map(BuildRequirementSet::One),
            _ => Some(BuildRequirementSet::All(self.requirements)),
        }
    }
}
