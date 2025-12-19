use std::fmt;

use destack_mir as mir;

/// Warning during Cranelift code generation.
#[derive(Debug, Clone)]
pub enum CodegenCraneliftWarning {
    /// Unexpected node.
    UnexpectedNode {
        node: mir::LocalNodeIdAny,
        wanted: mir::NodeType,
        message: Option<String>,
    },
}

impl CodegenCraneliftWarning {
    /// Get the node id of the warning, if available.
    pub fn node_id(&self) -> mir::LocalNodeIdAny {
        match self {
            Self::UnexpectedNode { node, .. } => *node,
        }
    }

    /// Get the warning message.
    pub fn message(&self) -> String {
        match self {
            Self::UnexpectedNode { message, .. } => message
                .clone()
                .unwrap_or_else(|| "unexpected node".to_string()),
        }
    }
}

impl fmt::Display for CodegenCraneliftWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
