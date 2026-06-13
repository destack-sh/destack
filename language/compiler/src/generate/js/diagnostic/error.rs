use destack_dir as dir;
use destack_js as js;

use crate::generate::js::ModuleLowerer;

/// Error during JS code generation.
#[derive(Debug, Clone)]
pub(crate) enum CodegenJsError {
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
    /// Missing type.
    MissingType {
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Internal error.
    Internal { message: String },
}

impl CodegenJsError {
    /// Get the error message.
    pub(crate) fn message(&self) -> String {
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
pub(crate) type CodegenJsResult<T> = Result<T, CodegenJsError>;

/// Extension methods for CodegenJsResult.
pub(crate) trait CodegenJsResultExt {
    /// Expect a node of the given type. Error with UnexpectedNode otherwise.
    fn expect_node<T: js::Node>(
        self,
        source_id: dir::GlobalNodeIdAny,
        lowerer: &mut ModuleLowerer<'_>,
    ) -> CodegenJsResult<js::LocalNodeId<T>>;
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
}
