use dyst_dir::{self as dir, Program};
use dyst_javascript_ast::NodeType;

use crate::TranspileDiagnostic;

/// Warning when transpiling something into JS/TS
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum TranspileWarning {
    /// Imprecise type.
    ImpreciseType { node: dir::GlobalNodeIdAny },
    /// Unexpected node.
    UnexpectedNode {
        node: dir::GlobalNodeIdAny,
        wanted: NodeType,
        message: Option<String>,
    },
    /// Expected statement, got something else.
    ExpectedStatement { node: dir::GlobalNodeIdAny },
}

impl TranspileWarning {
    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::ImpreciseType { .. } => "imprecise type".to_string(),
            Self::UnexpectedNode { node, wanted, .. } => {
                format!(
                    "unexpected {} (wanted {})",
                    node.local_id.ty.name(),
                    wanted.name()
                )
            }
            Self::ExpectedStatement { node, .. } => {
                format!("expected statement, got {}", node.local_id.ty.name())
            }
        }
    }

    /// Get the number of the warning.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ImpreciseType { .. } => 1,
            Self::UnexpectedNode { .. } => 2,
            Self::ExpectedStatement { .. } => 3,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::ImpreciseType { node, .. } => *node,
            Self::UnexpectedNode { node, .. } => *node,
            Self::ExpectedStatement { node, .. } => *node,
        }
    }

    /// Get the full code of the warning.
    pub fn full_code(&self) -> String {
        format!("TW{:03}", self.sub_code())
    }
}

impl From<TranspileWarning> for TranspileDiagnostic {
    fn from(warning: TranspileWarning) -> Self {
        TranspileDiagnostic::Warning(warning)
    }
}

impl std::fmt::Display for TranspileWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranspileWarning")
            .field("code", &format!("TW{:03}", self.sub_code()))
            .finish()
    }
}
