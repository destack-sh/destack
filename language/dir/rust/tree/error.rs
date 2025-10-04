use dyst_source::Span;

use crate::NodeType;

/// Error while lowering the DIR.
#[derive(Debug, Clone)]
pub struct DirError {
    /// The span of the error.
    pub span: Span,
    /// The (main) node type we tried to lower.
    pub node_type: Option<NodeType>,
    /// The source error.
    pub source: Option<Box<DirError>>,
}

impl DirError {
    /// Set the node type of the error.
    pub fn for_node_type(mut self, node_type: NodeType) -> Self {
        self.node_type = Some(node_type);
        self
    }
}

/// The result of a DIR lowering.
pub type DirResult<T> = Result<T, DirError>;

pub trait DirResultExt<T> {
    /// Set the node type of the error.
    fn for_node_type(self, node_type: NodeType) -> Result<T, DirError>;
}

impl<T> DirResultExt<T> for Result<T, DirError> {
    /// Set the node type of the error (if not already set)
    #[inline]
    fn for_node_type(self, node_type: NodeType) -> Self {
        if let Err(e) = &self
            && e.node_type.is_none()
        {
            Err(e.clone().for_node_type(node_type))
        } else {
            self
        }
    }
}
