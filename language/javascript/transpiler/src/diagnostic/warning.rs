use dyst_dir::{self as dir};

use crate::TranspileDiagnostic;

/// Warning when transpiling something into JS/TS
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum TranspileWarning {
    /// Unresolved expression.
    UnresolvedExpression { node: dir::NodeId<dir::Expression> } = 1,
    /// Unresolved path.
    UnresolvedPath {
        node: dir::NodeIdAny,
        path: dir::Path,
    } = 2,
    /// Unresolved type.
    UnresolvedType { node: dir::NodeId<dir::Type> } = 3,
}

impl TranspileWarning {
    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnresolvedExpression { .. } => "unresolved expression",
            Self::UnresolvedPath { .. } => "unresolved path",
            Self::UnresolvedType { .. } => "unresolved type",
        }
    }

    /// Get the number of the warning.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnresolvedExpression { .. } => 1,
            Self::UnresolvedPath { .. } => 2,
            Self::UnresolvedType { .. } => 3,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> dir::NodeIdAny {
        match self {
            Self::UnresolvedExpression { node } => node.into_any(),
            Self::UnresolvedPath { node, .. } => *node,
            Self::UnresolvedType { node } => *node,
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
