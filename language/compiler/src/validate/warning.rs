use dyst_dir::{Expression, NodeId, NodeIdAny, Session};

use crate::{CompileWarning, CompilerStage};

/// Warning when validating something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ValidateWarning {
    /// Missing type for an expression.
    MissingType { node: NodeId<Expression> } = 1,
}

impl ValidateWarning {
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
            Self::MissingType { node, .. } => Some(node.into_any()),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::MissingType { .. } => "missing type for an expression".to_string(),
        }
    }
}

impl std::fmt::Display for ValidateWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompilerStage::Validate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ValidateWarning> for CompileWarning {
    fn from(warning: ValidateWarning) -> Self {
        CompileWarning::Validate(warning)
    }
}
