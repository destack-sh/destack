use dyst_dir::{self as dir, Program};
use dyst_javascript_ast::{LocalNodeId, LocalNodeIdAny, Node, NodeType};

use crate::{TranspileDiagnostic, TranspileWarning, TranspilerUnit};

/// Error when transpiling something into JS/TS
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum TranspileError {
    /// Unsupported node.
    UnsupportedNode {
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Unexpected node.
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

impl TranspileError {
    /// Get the message of the error.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::UnsupportedNode { node, .. } => {
                format!("unsupported {}", node.local_id.ty.name())
            }
            Self::UnexpectedNode { node, wanted, .. } => {
                format!(
                    "unexpected {} (wanted {})",
                    node.local_id.ty.name(),
                    wanted.name()
                )
            }
            Self::UnresolvedNode { message, .. } => message
                .as_ref()
                .cloned()
                .unwrap_or("unresolved node".to_string()),
            Self::MissingType { message, .. } => message
                .as_ref()
                .cloned()
                .unwrap_or("missing type".to_string()),
        }
    }

    /// Get the number of the error.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 1,
            Self::UnexpectedNode { .. } => 2,
            Self::UnresolvedNode { .. } => 3,
            Self::MissingType { .. } => 4,
        }
    }

    /// Get the full code of the error. See CompileError.
    pub fn full_code(&self) -> String {
        format!("TE{:03}", self.sub_code())
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::UnsupportedNode { node, .. } => *node,
            Self::UnexpectedNode { node, .. } => *node,
            Self::UnresolvedNode { node, .. } => *node,
            Self::MissingType { node, .. } => *node,
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
    /// Expect a node of the given type. Error with UnexpectedNode otherwise.
    fn expect_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<T>>;

    /// Prefer a node of the given type. Warn with UnexpectedNode otherwise.
    fn prefer_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        unit: &mut TranspilerUnit,
    ) -> Option<LocalNodeId<T>>;

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        unit: &mut TranspilerUnit,
    ) -> Option<LocalNodeId<T>>;
}

impl TranspileResultExt for TranspileResult<LocalNodeIdAny> {
    fn expect_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                // check the node type matches the expected type
                if node_id.ty == T::TYPE {
                    Ok(LocalNodeId::<T>::new(node_id.id))
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

    fn prefer_node<T: Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        unit: &mut TranspilerUnit,
    ) -> Option<LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(LocalNodeId::<T>::new(node_id.id))
                } else {
                    unit.warning(TranspileWarning::UnexpectedNode {
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
        _unit: &mut TranspilerUnit,
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
