use crate::{Node, NodeType};
	
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
	Namespace {},
	Class {},
	Interface {},
	Enum {},
	Function {},
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}