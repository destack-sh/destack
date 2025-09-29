use crate::{Block, Expression, Match, Node, NodeId, NodeType};

/// A Try is try/catch statement.
/// The try expression may be a single statement or a block of statements.
/// Any error Result within the try expression aborts the try expression and:
///  1. If there is a catch, jumps to the catch pattern matching for handling.
///  2. If there is no catch, the error is propagated to the caller explicitly.
///
/// Examples:
/// ```
/// try fileOperation() // implicitly unwraps the Result, returns Error case
///
/// try { // implicitly unwraps all Results inside
///     let a = riskyOperationA() // a is Result.Ok(_) from riskyOperationA
///     riskyOperationB(a)
/// } // no catch needed if containing function has compatible Result type (Into suffices)
///
/// try { // explicitly unwraps all Results inside
///     ...
/// } catch e { // match all errors
///     NumericError(x) => Error(@format("bad number: {x}"))
///     FormatError => Error(@format("bad format {e}"))
///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Try {
    Expression {
        try_expression: NodeId<Expression>,
    },
    Block {
        try_block: NodeId<Block>,
    },
    BlockWithCatch {
        try_block: NodeId<Block>,
        catch_match: NodeId<Match>,
    },
}

impl Node for Try {
    const KIND: NodeType = NodeType::Try;
}
