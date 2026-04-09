use destack_artifact::{ArtifactKey, ArtifactStamp};

use crate::Change;

/// One artifact dependency needed before one provide attempt can continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequirement {
    /// The exact artifact key that is needed.
    pub key: ArtifactKey,
    /// The expected stamp for that key.
    pub stamp: ArtifactStamp,
}

/// One source edit needed before one provide attempt can continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRequirement {
    /// The source change needed to expand the revision.
    pub change: Change,
}

/// One concrete requirement yielded by one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement {
    /// One required artifact.
    Artifact(ArtifactRequirement),
    /// One required source edit.
    Source(SourceRequirement),
}

impl From<ArtifactRequirement> for Requirement {
    fn from(requirement: ArtifactRequirement) -> Self {
        Self::Artifact(requirement)
    }
}

impl From<SourceRequirement> for Requirement {
    fn from(requirement: SourceRequirement) -> Self {
        Self::Source(requirement)
    }
}

/// One yielded requirement set from a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementSet {
    /// One concrete requirement.
    One(Requirement),
    /// All listed concrete requirements.
    All(Vec<Requirement>),
}

impl RequirementSet {
    /// Build a requirement set for one concrete requirement.
    pub fn one(requirement: impl Into<Requirement>) -> Self {
        Self::One(requirement.into())
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

    /// Visit each artifact requirement in this set.
    pub fn for_each_artifact(&self, mut handle: impl FnMut(&ArtifactRequirement)) {
        self.for_each(|requirement| {
            if let Requirement::Artifact(requirement) = requirement {
                handle(requirement);
            }
        });
    }

    /// Return true when any source requirements are present.
    pub fn has_source_requirements(&self) -> bool {
        match self {
            Self::One(Requirement::Source(..)) => true,
            Self::One(Requirement::Artifact(..)) => false,
            Self::All(requirements) => requirements
                .iter()
                .any(|requirement| matches!(requirement, Requirement::Source(..))),
        }
    }
}

/// One outer provider result error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvideError<E> {
    /// The provider needs more source or artifacts first.
    Requirements(RequirementSet),
    /// The provider failed hard.
    Failed(E),
}
