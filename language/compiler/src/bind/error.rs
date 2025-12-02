use destack_ast::StringId;
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, ModuleId, Program, StaticKey};

use crate::{TaskDependency, TaskError, TaskPhase};

/// Error when binding something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindError {
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
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
}

impl TryFrom<BindError> for TaskDependency {
    type Error = BindError;

    fn try_from(error: BindError) -> Result<Self, Self::Error> {
        // interface required for tasks but binds cannot yield (because imports cannot yield)
        Err(error)
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
            Self::UnsupportedNode { .. } => 2,
            Self::ConflictingBinding { .. } => 3,
            Self::ConflictingExport { .. } => 4,
            Self::ConflictingDefaultExport { .. } => 5,
        }
    }

    /// Get the node id of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::UnsupportedNode { node } => *node,
            Self::ConflictingBinding { node, .. } => *node,
            Self::ConflictingExport { node, .. } => *node,
            Self::ConflictingDefaultExport { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
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
