use dyst_dir::{ModuleId, NodeIdAny, Session};

use crate::{CompileWarning, CompilerStage};

/// Warning when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveWarning {
    /// Unknown import.
    UnknownImport { module: ModuleId, node: NodeIdAny } = 1,
}

impl ResolveWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnknownImport { .. } => 1,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::UnknownImport { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::UnknownImport { .. } => "unknown import".to_string(),
        }
    }
}

impl std::fmt::Display for ResolveWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompilerStage::Resolve.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ResolveWarning> for CompileWarning {
    fn from(warning: ResolveWarning) -> Self {
        CompileWarning::Resolve(warning)
    }
}
