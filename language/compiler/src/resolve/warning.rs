use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{Phase, CompileWarning};

/// Warning when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveWarning {
    /// Unknown import.
    UnknownImport { node: GlobalNodeIdAny },
    /// Unused imports / unused re-exports.
    UnusedImport { node: GlobalNodeIdAny },
    /// Import that resolves but is only used for side effects.
    SideEffectOnlyImport { node: GlobalNodeIdAny },
}

impl ResolveWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnknownImport { .. } => 1,
            Self::UnusedImport { .. } => 2,
            Self::SideEffectOnlyImport { .. } => 3,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::UnknownImport { node, .. } => Some(*node),
            Self::UnusedImport { node, .. } => Some(*node),
            Self::SideEffectOnlyImport { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::UnknownImport { .. } => "unknown import".to_string(),
            Self::UnusedImport { .. } => "unused import".to_string(),
            Self::SideEffectOnlyImport { .. } => "side effect only import".to_string(),
        }
    }
}

impl std::fmt::Display for ResolveWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveWarning")
            .field(
                "code",
                &format!("W{}{:03}", Phase::Resolve.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ResolveWarning> for CompileWarning {
    fn from(warning: ResolveWarning) -> Self {
        CompileWarning::Resolve(warning)
    }
}
