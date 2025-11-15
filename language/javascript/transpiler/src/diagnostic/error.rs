use dyst_dir::{self as dir};

use crate::TranspileDiagnostic;

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
        wanted: dir::NodeType,
        message: Option<String>,
    } = 3,
}

impl TranspileError {
    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node",
            Self::UnsupportedPath { .. } => "unsupported path",
            Self::UnexpectedNode { .. } => "unexpected node",
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

pub trait TranspileResultExt {
    /// Expect a node of the given type.
    fn expect_node<T: dir::Node>(self) -> TranspileResult<dir::NodeId<T>>;

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: dir::Node>(self) -> Option<dir::NodeId<T>>;
}

impl TranspileResultExt for TranspileResult<dir::NodeIdAny> {
    /// Expect a node of the given type. Error with UnexpectedNode if the node type does not match.
    fn expect_node<T: dir::Node>(self) -> TranspileResult<dir::NodeId<T>> {
        match self {
            Ok(node_id) => {
                // check the node type matches the expected type
                if node_id.ty == T::TYPE {
                    Ok(dir::NodeId::<T>::new(node_id.id))
                } else {
                    Err(TranspileError::UnexpectedNode {
                        node: node_id,
                        wanted: T::TYPE,
                        message: None,
                    })
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: dir::Node>(self) -> Option<dir::NodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(dir::NodeId::<T>::new(node_id.id))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
