use dyst_dir::{NodeIdAny, Session};

use crate::{CompileWarning, CompilerStage};

/// Warning when optimizing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LinkWarning {
    /// Unknown type for an expression.
    MissingType { node: NodeIdAny } = 1,
}

impl LinkWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingType { .. } => 1,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::MissingType { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::MissingType { .. } => "unknown type for an expression".to_string(),
        }
    }
}

impl std::fmt::Display for LinkWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompilerStage::Link.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<LinkWarning> for CompileWarning {
    fn from(warning: LinkWarning) -> Self {
        CompileWarning::Link(warning)
    }
}
