use dyst_dir::{self as dir};
use dyst_javascript_ast::{Node, NodeId, NodeIdAny, NodeType};

use crate::{TranspileDiagnostic, TranspilerUnit};

/// Error when transpiling something into JS/TS
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum TranspileError {
    /// Unsupported node.
    UnsupportedNode {
        node: dir::NodeIdAny,
        message: Option<String>,
    } = 1,
    /// Unsupported path.
    UnsupportedPath {
        node: dir::NodeIdAny,
        path: dir::Path,
    } = 2,

    /// Unexpected node.
    UnexpectedNode {
        node: dir::NodeIdAny,
        wanted: NodeType,
        message: Option<String>,
    } = 3,
}

impl TranspileError {
    /// Get the message of the error.
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedNode { node, .. } => format!("unsupported {}", node.ty.name()),
            Self::UnsupportedPath { .. } => "unsupported path".to_string(),
            Self::UnexpectedNode { node, wanted, .. } => {
                format!("unexpected {} (wanted {})", node.ty.name(), wanted.name())
            }
        }
    }

    /// Get the number of the error.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 1,
            Self::UnsupportedPath { .. } => 2,
            Self::UnexpectedNode { .. } => 3,
        }
    }

    /// Get the full code of the error. See CompileError.
    pub fn full_code(&self) -> String {
        format!("TE{:03}", self.sub_code())
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> dir::NodeIdAny {
        match self {
            Self::UnsupportedNode { node, .. } => *node,
            Self::UnsupportedPath { node, .. } => *node,
            Self::UnexpectedNode { node, .. } => *node,
        }
    }
}

impl From<TranspileError> for TranspileDiagnostic {
    fn from(error: TranspileError) -> Self {
        TranspileDiagnostic::Error(error)
    }
}

pub type TranspileResult<T> = Result<T, TranspileError>;

/// Extension methods for TranspileResult.
pub trait TranspileResultExt {
    /// Expect a node of the given type.
    fn expect_node<T: Node>(
        self,
        source_id: dir::NodeIdAny,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<T>>;

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: Node>(
        self,
        source_id: dir::NodeIdAny,
        unit: &mut TranspilerUnit,
    ) -> Option<NodeId<T>>;
}

impl TranspileResultExt for TranspileResult<NodeIdAny> {
    /// Expect a node of the given type. Error with UnexpectedNode if the node type does not match.
    fn expect_node<T: Node>(
        self,
        source_id: dir::NodeIdAny,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<T>> {
        match self {
            Ok(node_id) => {
                // check the node type matches the expected type
                if node_id.ty == T::TYPE {
                    Ok(NodeId::<T>::new(node_id.id))
                } else {
                    Err(TranspileError::UnexpectedNode {
                        node: source_id,
                        wanted: T::TYPE,
                        message: None,
                    })
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: Node>(
        self,
        _source_id: dir::NodeIdAny,
        _unit: &mut TranspilerUnit,
    ) -> Option<NodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(NodeId::<T>::new(node_id.id))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
