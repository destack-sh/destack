use destack_dir as dir;
use destack_js as js;

use crate::{CodegenJsWarning, ModuleLowerer};

/// Error during JS code generation.
#[derive(Debug, Clone)]
pub enum CodegenJsError {
    /// Unsupported target/output format.
    UnsupportedTarget {
        format: String,
        message: Option<String>,
    },
    /// Unsupported construct.
    UnsupportedConstruct {
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Unexpected node type.
    UnexpectedNode {
        node: dir::GlobalNodeIdAny,
        wanted: js::NodeType,
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
    /// Internal error.
    Internal { message: String },
}

impl CodegenJsError {
    /// Get the node id of the error, if available.
    pub fn node_id(&self) -> Option<dir::GlobalNodeIdAny> {
        match self {
            Self::UnsupportedTarget { .. } => None,
            Self::UnsupportedConstruct { node, .. } => Some(*node),
            Self::UnexpectedNode { node, .. } => Some(*node),
            Self::UnresolvedNode { node, .. } => Some(*node),
            Self::MissingType { node, .. } => Some(*node),
            Self::Internal { .. } => None,
        }
    }

    /// Get the error message.
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedTarget { format, message } => message
                .clone()
                .unwrap_or_else(|| format!("unsupported target: {format}")),
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
            Self::Internal { message } => format!("internal error: {message}"),
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
    fn expect_node<T: js::Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> CodegenJsResult<js::LocalNodeId<T>>;

    /// Prefer a node of the given type. Warn with UnexpectedNode otherwise.
    fn prefer_node<T: js::Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<js::LocalNodeId<T>>;

    /// Unwrap a node of the given type. None otherwise.
    fn unwrap_node<T: js::Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<js::LocalNodeId<T>>;
}

impl CodegenJsResultExt for CodegenJsResult<js::LocalNodeIdAny> {
    fn expect_node<T: js::Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> CodegenJsResult<js::LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Ok(js::LocalNodeId::<T>::new(node_id.id))
                } else {
                    let source_kind = if source_id.local_id.ty == dir::NodeType::Expression {
                        let expression_id =
                            dir::LocalNodeId::<dir::Expression>::new(source_id.local_id.id);
                        let _expression = lowerer.dir_tree.get(expression_id);
                        " for expression".to_string()
                    } else {
                        String::new()
                    };

                    Err(CodegenJsError::UnexpectedNode {
                        node: source_id,
                        wanted: T::TYPE,
                        message: Some(format!(
                            "lowered to unexpected {} (wanted {}){source_kind}",
                            node_id.ty.name(),
                            T::TYPE.name()
                        )),
                    })
                }
            }
            Err(error) => Err(error),
        }
    }

    fn prefer_node<T: js::Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<js::LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(js::LocalNodeId::<T>::new(node_id.id))
                } else {
                    lowerer.warning(CodegenJsWarning::UnexpectedNode {
                        node: source_id,
                        wanted: T::TYPE,
                        message: Some(format!(
                            "lowered to unexpected {} (wanted {})",
                            node_id.ty.name(),
                            T::TYPE.name()
                        )),
                    });
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn unwrap_node<T: js::Node>(
        self,
        _source_id: dir::GlobalNodeIdAny,
        _lowerer: &mut ModuleLowerer<'_>,
    ) -> Option<js::LocalNodeId<T>> {
        match self {
            Ok(node_id) => {
                if node_id.ty == T::TYPE {
                    Some(js::LocalNodeId::<T>::new(node_id.id))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
