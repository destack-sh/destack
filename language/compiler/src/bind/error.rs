use dyst_ast::StringId;
use dyst_dir::{GlobalNodeIdAny, GlobalScopeId, GlobalSymbolId, Program};

use crate::{CompileError, CompilePhase, CompileTaskWait};

// nocheckin: bind exports, report duplicate declaration bindings, ..

/// Error when binding something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskWait },
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
    /// Conflicting declarations in the same scope.
    ConflictingDeclaration {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        name: StringId,
    },
    /// Conflicting parameter or pattern binding.
    ConflictingBinding {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        name: StringId,
    },
    /// Conflicting export name in the same module.
    ConflictingExport {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        name: StringId,
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
            Self::ConflictingDeclaration { .. } => 2,
            Self::ConflictingBinding { .. } => 3,
            Self::ConflictingExport { .. } => 4,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { wait } => wait.nodes.first().copied(),
            Self::UnsupportedNode { node } => Some(*node),
            Self::ConflictingDeclaration { node, .. } => Some(*node),
            Self::ConflictingBinding { node, .. } => Some(*node),
            Self::ConflictingExport { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Wait { .. } => "wait for task".to_string(),
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
            Self::ConflictingDeclaration { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("conflicting declaration of '{name}'")
            }
            Self::ConflictingBinding { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("conflicting binding of '{name}'")
            }
            Self::ConflictingExport { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("conflicting export of '{name}'")
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
