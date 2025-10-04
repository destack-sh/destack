/// A Module is a module declaration node.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {}

impl Node for Module {
    const KIND: NodeType = NodeType::Module;
}
