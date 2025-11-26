use dyst_ast::StringId;
use dyst_dir::{GlobalNodeIdAny, GlobalScopeId, ModuleId, Program};

use crate::{CompileError, CompilePhase, CompileTaskWait};

/// Error when binding something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskWait },
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
    /// Conflicting symbol binding.
    ConflictingBinding {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        name: StringId,
    },
    /// Conflicting export name in the same module.
    ConflictingExport {
        node: GlobalNodeIdAny,
        other_node: Option<GlobalNodeIdAny>,
        module: ModuleId,
        name: StringId,
    },
    /// Conflicting default export.
    ConflictingDefaultExport {
        node: GlobalNodeIdAny,
        other_node: Option<GlobalNodeIdAny>,
        name: Option<StringId>,
        module: ModuleId,
    },
}

pub type BindResult<T> = Result<T, BindError>;

impl From<BindError> for CompileError {
    #[inline]
    fn from(error: BindError) -> Self {
        CompileError::Bind(error)
    }
}

impl BindError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Wait { .. } => 0,
            Self::UnsupportedNode { .. } => 1,
            Self::ConflictingBinding { .. } => 2,
            Self::ConflictingExport { .. } => 3,
            Self::ConflictingDefaultExport { .. } => 4,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { wait } => wait.nodes.first().copied(),
            Self::UnsupportedNode { node } => Some(*node),
            Self::ConflictingBinding { node, .. } => Some(*node),
            Self::ConflictingExport { node, .. } => Some(*node),
            Self::ConflictingDefaultExport { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Wait { .. } => "wait for task".to_string(),
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
            Self::ConflictingBinding { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("conflicting binding of '{name}'")
            }
            Self::ConflictingExport { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("conflicting export of '{name}'")
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
                &format!("E{}{:03}", CompilePhase::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}
