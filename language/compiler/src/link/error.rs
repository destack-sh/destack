use destack_dir::GlobalNodeIdAny;

use crate::{TaskDependency, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when linking something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LinkError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Missing target.
    MissingTarget {
        node: GlobalNodeIdAny,
        target: String,
    },
    /// Invalid target configuration.
    InvalidTarget {
        node: GlobalNodeIdAny,
        target: String,
        message: String,
    },
    /// Internal error during linking.
    Internal {
        node: GlobalNodeIdAny,
        message: String,
    },
}

impl TryFrom<LinkError> for TaskDependency {
    type Error = LinkError;

    fn try_from(error: LinkError) -> Result<Self, Self::Error> {
        match error {
            LinkError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl LinkError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::MissingTarget { .. } => 3,
            Self::InvalidTarget { .. } => 4,
            Self::Internal { .. } => 2,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::UnsatisfiedDependency { dependency } => dependency.node(),
            Self::MissingTarget { node, .. } => *node,
            Self::InvalidTarget { node, .. } => *node,
            Self::Internal { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::MissingTarget { target, .. } => format!("missing target: {target}"),
            Self::InvalidTarget {
                target, message, ..
            } => {
                format!("invalid target: {target}: {message}")
            }
            Self::Internal { message, .. } => message.clone(),
        }
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Link.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type LinkResult<T> = Result<T, LinkError>;

impl From<LinkError> for TaskError {
    #[inline]
    fn from(error: LinkError) -> Self {
        TaskError::Link(error)
    }
}
