use destack_artifact::ArtifactVersion;
use destack_workspace::Change;

use crate::{CompileError, DiagnosticAnchor};

/// Requirement for one artifact key.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRequirement {
    /// The diagnostic anchor for this requirement.
    pub anchor: DiagnosticAnchor,
    /// The exact required artifact version.
    pub version: ArtifactVersion,
    /// Optional fallback error if the requirement cannot be satisfied.
    pub error: Option<Box<CompileError>>,
}

impl ArtifactRequirement {
    /// Create a new artifact requirement.
    pub fn new(anchor: DiagnosticAnchor, version: ArtifactVersion) -> Self {
        Self {
            anchor,
            version,
            error: None,
        }
    }

    /// Attach a fallback error to this requirement.
    pub fn with_error(self, error: CompileError) -> Self {
        Self {
            error: Some(Box::new(error)),
            ..self
        }
    }
}

/// Requirement for one file world update.
#[derive(Debug, Clone, PartialEq)]
pub struct FileRequirement {
    /// The diagnostic anchor for this requirement.
    pub anchor: DiagnosticAnchor,
    /// The source change needed to expand the source world.
    pub change: Change,
    /// Optional fallback error if the requirement cannot be satisfied.
    pub error: Option<Box<CompileError>>,
}

impl FileRequirement {
    /// Create a new file requirement.
    pub fn new(anchor: DiagnosticAnchor, change: Change) -> Self {
        Self {
            anchor,
            change,
            error: None,
        }
    }

    /// Attach a fallback error to this requirement.
    pub fn with_error(self, error: CompileError) -> Self {
        Self {
            error: Some(Box::new(error)),
            ..self
        }
    }
}

/// One concrete compiler requirement.
#[derive(Debug, Clone, PartialEq)]
pub enum Requirement {
    /// One required artifact.
    Artifact(ArtifactRequirement),
    /// One required file update.
    File(FileRequirement),
}

impl Requirement {
    /// Return the diagnostic anchor for this requirement.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Artifact(requirement) => requirement.anchor.clone(),
            Self::File(requirement) => requirement.anchor.clone(),
        }
    }

    /// Return the fallback error for this requirement when present.
    pub fn fallback_error(&self) -> Option<&CompileError> {
        match self {
            Self::Artifact(requirement) => requirement.error.as_deref(),
            Self::File(requirement) => requirement.error.as_deref(),
        }
    }
}

impl From<ArtifactRequirement> for Requirement {
    fn from(requirement: ArtifactRequirement) -> Self {
        Self::Artifact(requirement)
    }
}

impl From<FileRequirement> for Requirement {
    fn from(requirement: FileRequirement) -> Self {
        Self::File(requirement)
    }
}

/// Requirement algebra for the unified compiler graph.
#[derive(Debug, Clone, PartialEq)]
pub enum RequirementSet {
    /// Require one concrete requirement.
    One(Requirement),
    /// Require all listed concrete requirements.
    All(Vec<Requirement>),
}

impl RequirementSet {
    /// Create a requirement set for one concrete requirement.
    pub fn one(requirement: impl Into<Requirement>) -> Self {
        Self::One(requirement.into())
    }

    /// Return the first concrete requirement for diagnostics and tracing.
    pub fn first(&self) -> Option<&Requirement> {
        match self {
            Self::One(requirement) => Some(requirement),
            Self::All(requirements) => requirements.first(),
        }
    }

    /// Return the diagnostic anchor for this requirement set.
    pub fn anchor(&self) -> DiagnosticAnchor {
        self.first()
            .map(Requirement::anchor)
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
    pub fn for_each(&self, mut handle: impl FnMut(&Requirement)) {
        match self {
            Self::One(requirement) => handle(requirement),
            Self::All(requirements) => {
                for requirement in requirements {
                    handle(requirement);
                }
            }
        }
    }

    /// Visit each concrete artifact requirement in this set.
    pub fn for_each_artifact(&self, mut handle: impl FnMut(&ArtifactRequirement)) {
        self.for_each(|requirement| {
            if let Requirement::Artifact(requirement) = requirement {
                handle(requirement);
            }
        });
    }

    /// Visit each concrete file requirement in this set.
    pub fn for_each_file(&self, mut handle: impl FnMut(&FileRequirement)) {
        self.for_each(|requirement| {
            if let Requirement::File(requirement) = requirement {
                handle(requirement);
            }
        });
    }

    /// Return true when any concrete requirement matches the predicate.
    pub fn any(&self, mut predicate: impl FnMut(&Requirement) -> bool) -> bool {
        match self {
            Self::One(requirement) => predicate(requirement),
            Self::All(requirements) => requirements.iter().any(predicate),
        }
    }

    /// Return true when all concrete requirements match the predicate.
    pub fn all(&self, mut predicate: impl FnMut(&Requirement) -> bool) -> bool {
        match self {
            Self::One(requirement) => predicate(requirement),
            Self::All(requirements) => requirements.iter().all(predicate),
        }
    }

    /// Return the first mapped value from the concrete requirements.
    pub fn find_map<T>(&self, mut handle: impl FnMut(&Requirement) -> Option<T>) -> Option<T> {
        match self {
            Self::One(requirement) => handle(requirement),
            Self::All(requirements) => requirements.iter().find_map(handle),
        }
    }

    /// Return true when this set contains any file requirements.
    pub fn has_file_requirements(&self) -> bool {
        self.any(|requirement| matches!(requirement, Requirement::File(..)))
    }

    /// Return all file requirements in this set.
    pub fn file_requirements(&self) -> Vec<FileRequirement> {
        let mut requirements = Vec::new();

        self.for_each(|requirement| {
            if let Requirement::File(requirement) = requirement {
                requirements.push(requirement.clone());
            }
        });

        requirements
    }

    /// Convert this requirement set into a flat vector.
    pub fn into_requirements(self) -> Vec<Requirement> {
        match self {
            Self::One(requirement) => vec![requirement],
            Self::All(requirements) => requirements,
        }
    }
}

/// Error when one requirement is not satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum RequirementError {
    /// Requirement is not yet available, need to yield.
    NotReady { requirement: RequirementSet },
    /// Upstream requirement build has failed.
    Failed { requirement: RequirementSet },
}

impl RequirementError {
    /// Return the carried requirement.
    pub fn requirement(&self) -> &RequirementSet {
        match self {
            Self::NotReady { requirement } | Self::Failed { requirement } => requirement,
        }
    }

    /// Convert into the carried requirement.
    pub fn into_requirement(self) -> RequirementSet {
        match self {
            Self::NotReady { requirement } | Self::Failed { requirement } => requirement,
        }
    }
}

impl TryFrom<RequirementError> for RequirementSet {
    type Error = RequirementError;

    fn try_from(error: RequirementError) -> Result<Self, Self::Error> {
        match error {
            RequirementError::NotReady { requirement } => Ok(requirement),
            RequirementError::Failed { .. } => Err(error),
        }
    }
}

/// Collector for coalescing requirements from multiple operations.
#[derive(Debug, Default)]
pub struct RequirementCollector {
    requirements: Vec<Requirement>,
}

impl RequirementCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self {
            requirements: Vec::new(),
        }
    }

    /// Collect a result as a requirement.
    pub fn try_collect<T, E>(&mut self, result: Result<T, E>) -> Option<E>
    where
        E: TryInto<RequirementSet, Error = E>,
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
    pub fn try_into_requirement(self) -> Option<RequirementSet> {
        match self.requirements.len() {
            0 => None,
            1 => self
                .requirements
                .into_iter()
                .next()
                .map(RequirementSet::One),
            _ => Some(RequirementSet::All(self.requirements)),
        }
    }
}
