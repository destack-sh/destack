use crate::{LocalNodeId, LocalNodeIdAny, ModuleLowerer, Node, NodeType};
use destack_dir as dir;

/// Error during JS code generation.
#[derive(Debug, Clone)]
pub enum CodegenJsError {
    /// Unsupported construct.
    UnsupportedConstruct {
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Unexpected node type.
    UnexpectedNode {
        node: dir::GlobalNodeIdAny,
        wanted: NodeType,
        message: Option<String>,
    },
    /// Unresolved node.
    UnresolvedNode {
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Missing type.
    MissingType {
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    },
}

impl CodegenJsError {
    /// Get the node id of the error.
    pub fn node_id(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::UnsupportedConstruct { node, .. } => *node,
            Self::UnexpectedNode { node, .. } => *node,
            Self::UnresolvedNode { node, .. } => *node,
            Self::MissingType { node, .. } => *node,
        }
    }

    /// Get the error message.
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedConstruct { node, message } => message
                .clone()
                .unwrap_or_else(|| format!("unsupported {}", node.local_id.ty.name())),
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
            Self::UnresolvedNode { message, .. } => message
                .clone()
                .unwrap_or_else(|| "unresolved node".to_string()),
            Self::MissingType { message, .. } => message
                .clone()
                .unwrap_or_else(|| "missing type".to_string()),
        }
    }
}

impl std::fmt::Display for CodegenJsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CodegenJsError {}

/// Result type for JS codegen operations.
pub type CodegenJsResult<T> = Result<T, CodegenJsError>;

/// Extension methods for CodegenJsResult.
pub trait CodegenJsResultExt {
    /// Expect a node of the given type. Error with UnexpectedNode otherwise.
    fn expect_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> CodegenJsResult<LocalNodeId<T>>;

    /// Prefer a node of the given type. Warn with UnexpectedNode otherwise.
    fn prefer_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<LocalNodeId<T>>;

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<LocalNodeId<T>>;
}

impl CodegenJsResultExt for CodegenJsResult<LocalNodeIdAny> {
    fn expect_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        _lowerer: &mut ModuleLowerer<'_>,
    ) -> CodegenJsResult<LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Ok(LocalNodeId::<T>::new(node_id.id))
                } else {
                    Err(CodegenJsError::UnexpectedNode {
                        node: source_id,
                        wanted: T::TYPE,
                        message: None,
                    })
                }
            }
            Err(error) => Err(error),
        }
    }

    fn prefer_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(LocalNodeId::<T>::new(node_id.id))
                } else {
                    lowerer.warning(crate::CodegenJsWarning::UnexpectedNode {
                        node: source_id,
                        wanted: T::TYPE,
                        message: None,
                    });
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn unwrap_node<T: Node>(
        self,
        _source_id: dir::GlobalNodeIdAny,
        _lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(LocalNodeId::<T>::new(node_id.id))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
