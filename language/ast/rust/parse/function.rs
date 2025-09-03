//! Parse functions and closures.

use crate::{Expression, FunctionSignature, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a Function or "lambda" definition or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    ///
    /// Examples:
    /// ```
    /// function @baz(a: int32, b: boolean) => MyStruct, boolean {
    ///    ...
    /// }
    ///
    /// // optional , if newline-delimited
    /// function longBar(
    ///   /// doc comment for `a`
    ///   a: int32
    ///   /// doc comment for `b`
    ///   b: boolean
    ///   c: Vector2
    /// ) => int32, boolean {
    ///    ...
    /// )
    ///
    /// // lambda style
    ///
    /// () => { 0 } // no function keyword
    /// () => None // slightly ambiguous but returns None
    /// (x: int32) => x + 1
    ///
    /// // for return type in lambdas, you need a `{ ... }` body
    /// (a: int32, b: int32) => int32 {
    ///      let y = someFunction(a, b)
    ///      y + 4
    /// }
    /// ```
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
