use crate::{Node, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyItem {}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}
