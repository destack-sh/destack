use crate::{Block, Expression, Match, Node, NodeId, NodeType, Pattern, Runtime, StringId};

/// A While is while loop.
///
/// Examples:
/// ```
/// @while x > 1 {
///     y = 2
/// }
///
/// while y < 10 l: {
///     y = 2
///     break :l
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct While {
    /// The runtime of the while loop.
    pub runtime: Option<Runtime>,
    /// The condition of the while loop.
    pub condition: NodeId<Expression>,
    /// The body of the while loop.
    pub body: NodeId<Block>,
}

impl Node for While {
    const KIND: NodeType = NodeType::While;
}

/// A For is a for loop over an iterator with a pattern.
///
/// Examples:
/// ```
/// @for x in 1..10 {
///     y = 2
/// }
///
/// for x in 1..10 a: {
///     if y > 5 {
///         continue :a
///     }
///     y = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct For {
    /// The runtime of the for loop.
    pub runtime: Option<Runtime>,
    /// The pattern to match the iterator with (e.g., `x`).
    pub pattern: NodeId<Pattern>,
    /// The iterator to iterate over (e.g., `1..10`).
    pub iterator: NodeId<Expression>,
    /// The body of the for loop.
    pub body: NodeId<Block>,
}

impl Node for For {
    const KIND: NodeType = NodeType::For;
}

/// A Loop is an unconditional loop.
///
/// Examples:
/// ```
/// loop {
///     y = getNext()
///     if y < 0 {
///         break
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    /// The runtime of the loop.
    pub runtime: Option<Runtime>,
    /// The body of the loop.
    pub body: NodeId<Block>,
}

impl Node for Loop {
    const KIND: NodeType = NodeType::Loop;
}

/// A Break is break statement.
///
/// Examples:
/// ```
/// break
/// break :label
/// break :label 17
/// break 15
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Break {
    /// The label to break to (e.g., `:label`).
    pub label: Option<StringId>,
    /// The value to break with (e.g., `17`).
    pub value: Option<NodeId<Expression>>,
}

impl Node for Break {
    const KIND: NodeType = NodeType::Break;
}

/// A Continue is continue statement.
///
/// Examples:
/// ```
/// continue
/// continue :label
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Continue {
    /// The label to continue to (e.g., `:label`).
    pub label: Option<StringId>,
}

impl Node for Continue {
    const KIND: NodeType = NodeType::Continue;
}

/// Defer expression until scope exit.
///
/// Examples:
/// ```
/// defer someFunction()
///
/// defer {
///     someFunction()
///     someOtherFunction()
/// }
///
/// defer :label {
///     someOtherFunction()
/// }
///
/// defer catch e {
///     _ => someErrorHandler(e)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Defer {
    /// Defer a single expression.
    Expression(NodeId<Expression>),
    /// Defer a block of statements.
    Block(NodeId<Block>),
    /// Defer catch with matching.
    Catch(NodeId<Match>),
}

impl Node for Defer {
    const KIND: NodeType = NodeType::Defer;
}

/// Return expression.
///
/// Examples:
/// ```
/// return
/// return 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Return {
    pub value: Option<NodeId<Expression>>,
}

impl Node for Return {
    const KIND: NodeType = NodeType::Return;
}
