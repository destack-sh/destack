//! Parse functions and closures.

use crate::{Expression, FunctionSignature, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    pub fn eat_function(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    /// Eat a function signature.
    ///
    /// Examples:
    /// ```
    /// () => int32
    /// (int32) => (int32, int32) // explicit tuple return type
    /// (int32) => int32, int32 // implicit tuple return type
    /// ```
    pub fn eat_function_signature(&mut self) -> ParseResult<NodeId<FunctionSignature>> {
        todo!()
    }
}
