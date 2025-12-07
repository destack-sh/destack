use std::fmt;

use destack_mir as mir;

/// Warning during Cranelift code generation.
#[derive(Debug, Clone)]
pub enum CodegenCraneliftWarning {
    /// Unoptimized code path.
    UnoptimizedCodePath {
        node: mir::LocalNodeIdAny,
        message: Option<String>,
    },
    /// Potential performance issue.
    PerformanceHint {
        node: mir::LocalNodeIdAny,
        message: Option<String>,
    },
}

impl CodegenCraneliftWarning {
    /// Get the node id of the warning, if available.
    pub fn node_id(&self) -> mir::LocalNodeIdAny {
        match self {
            Self::UnoptimizedCodePath { node, .. } => *node,
            Self::PerformanceHint { node, .. } => *node,
        }
    }

    /// Get the warning message.
    pub fn message(&self) -> String {
        match self {
            Self::UnoptimizedCodePath { message, .. } => message
                .clone()
                .unwrap_or_else(|| "unoptimized code path".to_string()),
            Self::PerformanceHint { message, .. } => message
                .clone()
                .unwrap_or_else(|| "potential performance issue".to_string()),
        }
    }
}

impl fmt::Display for CodegenCraneliftWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
