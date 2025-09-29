use crate::{Block, Expression, Node, NodeId, NodeType, Runtime};

/// If/then/else expression.
/// Then and else must be blocks.
///
/// Examples:
/// ```
/// // if
/// @if x > 0 {
///     print("positive")
/// }
///
/// // if else
/// if x > 0 {
///     print("positive")
/// } else {
///     print("not positive")
/// }
///
/// // if else if
/// if x > 0 {
///     print("positive")
/// } else if x == 0 {
///     print("zero")
/// } else {
///     print("negative")
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum If {
    // `if` with then block.
    If {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
    },
    // `if` with then block and else block.
    IfElse {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: NodeId<Block>,
    },
    // `if` with then block and else if block.
    IfElseIf {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_if: NodeId<If>,
    },
}

impl Node for If {
    const KIND: NodeType = NodeType::If;
}
