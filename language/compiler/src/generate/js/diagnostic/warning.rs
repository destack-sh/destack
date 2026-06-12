use destack_dir as dir;
use destack_js as js;

/// Warning during JS code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenJsWarning {
    /// Imprecise type.
    ImpreciseType { node: dir::GlobalNodeIdAny },
    /// Unexpected node.
    UnexpectedNode {
        node: dir::GlobalNodeIdAny,
        wanted: js::NodeType,
        message: Option<String>,
    },
    /// Expected statement, got something else.
    ExpectedStatement { node: dir::GlobalNodeIdAny },
}

impl CodegenJsWarning {
    /// Get the node id of the warning.
    pub fn node_id(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::ImpreciseType { node, .. } => *node,
            Self::UnexpectedNode { node, .. } => *node,
            Self::ExpectedStatement { node, .. } => *node,
        }
    }

    /// Get the warning message.
    pub fn message(&self) -> String {
        match self {
            Self::ImpreciseType { .. } => "imprecise type".to_string(),
            Self::UnexpectedNode {
                node,
                wanted,
                message,
            } => message.clone().unwrap_or_else(|| {
                format!(
                    "unexpected {} (wanted {})",
                    node.local_id.ty.name(),
                    wanted.name()
                )
            }),
            Self::ExpectedStatement { node, .. } => {
                format!("expected statement, got {}", node.local_id.ty.name())
            }
        }
    }
}

impl std::fmt::Display for CodegenJsWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}
