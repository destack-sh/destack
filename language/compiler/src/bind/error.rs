use destack_ast::StringId;
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, StaticKey};
use destack_source::ModuleId;
use destack_workspace::Program;

use crate::{DiagnosticAnchor, TaskDependency, TaskDependencyError, TaskError, TaskPhase};

/// Error when binding something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindError {
    /// Yield to a dependency.
    Yield { dependency: TaskDependency },
    /// Unsatisfied dependency (dependency failed).
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
    /// Conflicting symbol binding.
    ConflictingBinding {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        name: Option<StaticKey>,
    },
    /// Conflicting export name in the same module.
    ConflictingExport {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        module: ModuleId,
        name: Option<StaticKey>,
    },
    /// Conflicting default export.
    ConflictingDefaultExport {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        name: Option<StringId>,
        module: ModuleId,
    },
    /// Break used outside of a valid breakable context, or with an invalid label.
    IllegalBreak {
        node: GlobalNodeIdAny,
        label: Option<StringId>,
    },
    /// Continue used outside of a loop, or with an invalid label.
    IllegalContinue {
        node: GlobalNodeIdAny,
        label: Option<StringId>,
    },
    /// Referenced label does not exist in this scope.
    UnknownLabel {
        node: GlobalNodeIdAny,
        label: StringId,
    },
    /// Await used in an invalid context.
    IllegalAwait { node: GlobalNodeIdAny },
    /// Yield used in an invalid context.
    IllegalYield { node: GlobalNodeIdAny },
    /// Reserved identifier used in a forbidden context.
    ReservedIdentifier {
        node: GlobalNodeIdAny,
        name: StringId,
    },
    /// Directive prologue is invalid or cannot be interpreted.
    InvalidPrologue {
        node: GlobalNodeIdAny,
        content: String,
    },
}

impl From<TaskDependencyError> for BindError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<BindError> for TaskDependency {
    type Error = BindError;

    fn try_from(error: BindError) -> Result<Self, Self::Error> {
        match error {
            BindError::Yield { dependency } => Ok(dependency),
            other => Err(other),
        }
    }
}

pub type BindResult<T> = Result<T, BindError>;

impl From<BindError> for TaskError {
    #[inline]
    fn from(error: BindError) -> Self {
        TaskError::Bind(error)
    }
}

impl BindError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 1,
            Self::UnsatisfiedDependency { .. } => 2,
            Self::UnsupportedConstruct { .. } => 3,
            Self::ConflictingBinding { .. } => 4,
            Self::ConflictingExport { .. } => 5,
            Self::ConflictingDefaultExport { .. } => 6,
            Self::IllegalBreak { .. } => 7,
            Self::IllegalContinue { .. } => 8,
            Self::UnknownLabel { .. } => 9,
            Self::IllegalAwait { .. } => 10,
            Self::IllegalYield { .. } => 11,
            Self::ReservedIdentifier { .. } => 12,
            Self::InvalidPrologue { .. } => 13,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Yield { dependency } => dependency.anchor(),
            Self::UnsatisfiedDependency { dependency } => dependency.anchor(),
            Self::UnsupportedConstruct { node } => DiagnosticAnchor::Node(*node),
            Self::ConflictingBinding { node, .. } => DiagnosticAnchor::Node(*node),
            Self::ConflictingExport { node, .. } => DiagnosticAnchor::Node(*node),
            Self::ConflictingDefaultExport { node, .. } => DiagnosticAnchor::Node(*node),
            Self::IllegalBreak { node, .. } => DiagnosticAnchor::Node(*node),
            Self::IllegalContinue { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UnknownLabel { node, .. } => DiagnosticAnchor::Node(*node),
            Self::IllegalAwait { node, .. } => DiagnosticAnchor::Node(*node),
            Self::IllegalYield { node, .. } => DiagnosticAnchor::Node(*node),
            Self::ReservedIdentifier { node, .. } => DiagnosticAnchor::Node(*node),
            Self::InvalidPrologue { node, .. } => DiagnosticAnchor::Node(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Yield { .. } => "bind task yielded".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedConstruct { .. } => "unsupported construct".to_string(),
            Self::ConflictingBinding { name, .. } => {
                let name = name
                    .map(|name| name.name())
                    .flatten()
                    .map(|name| program.strings.get(name).to_string());
                if let Some(name) = name {
                    format!("conflicting binding of '{name}'")
                } else {
                    "conflicting binding".to_string()
                }
            }
            Self::ConflictingExport { name, .. } => {
                let name = name
                    .map(|name| name.name())
                    .flatten()
                    .map(|name| program.strings.get(name).to_string());
                if let Some(name) = name {
                    format!("conflicting export of '{name}'")
                } else {
                    "conflicting export".to_string()
                }
            }
            Self::ConflictingDefaultExport { name, .. } => {
                if let Some(name) = name {
                    let name = program.strings.get(*name).to_string();
                    format!("conflicting default export of '{name}'")
                } else {
                    "conflicting default export".to_string()
                }
            }
            Self::IllegalBreak { label, .. } => {
                if let Some(label) = label {
                    let label = program.strings.get(*label).to_string();
                    format!("illegal break to '{label}'")
                } else {
                    "illegal break".to_string()
                }
            }
            Self::IllegalContinue { label, .. } => {
                if let Some(label) = label {
                    let label = program.strings.get(*label).to_string();
                    format!("illegal continue to '{label}'")
                } else {
                    "illegal continue".to_string()
                }
            }
            Self::UnknownLabel { label, .. } => {
                let label = program.strings.get(*label).to_string();
                format!("unknown label '{label}'")
            }
            Self::IllegalAwait { .. } => "illegal await".to_string(),
            Self::IllegalYield { .. } => "illegal yield".to_string(),
            Self::ReservedIdentifier { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("reserved identifier '{name}'")
            }
            Self::InvalidPrologue { .. } => "invalid directive prologue".to_string(),
        }
    }
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}
