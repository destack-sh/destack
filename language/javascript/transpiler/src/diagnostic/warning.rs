use dyst_dir::{self as dir};

use crate::TranspileDiagnostic;

/// Warning when transpiling something into JS/TS
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum TranspileWarning {
    /// Unevaluated expression.
    UnevaluatedExpression { node: dir::NodeId<dir::Expression> } = 1,
    /// Unevaluated path.
    UnevaluatedPath {
        node: dir::NodeIdAny,
        path: dir::Path,
    } = 2,
}

impl TranspileWarning {
    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnevaluatedExpression { .. } => "unevaluated expression",
            Self::UnevaluatedPath { .. } => "unevaluated path",
        }
    }

    /// Get the number of the warning.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnevaluatedExpression { .. } => 1,
            Self::UnevaluatedPath { .. } => 2,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> dir::NodeIdAny {
        match self {
            Self::UnevaluatedExpression { node } => node.into_any(),
            Self::UnevaluatedPath { node, .. } => *node,
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
