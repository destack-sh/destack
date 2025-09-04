//! Parse functions and closures.

use destack_language_token::TokenType;

use crate::{
    Function, FunctionRuntime, FunctionSignature, FunctionStyle, Keyword, NodeId, ParseResult,
    Parser,
};

impl<'a> Parser<'a> {
    /// Eat a Function or "lambda" definition or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    ///
    /// Examples:
    /// ```
    /// // function style
    ///
    /// function @baz(a: int32, b: boolean) => MyStruct, boolean {
    ///      ...
    /// }
    ///
    /// // optional `,` for arguments and return type if newline-delimited
    /// function longBar(
    ///     /// doc comment for `a`
    ///     a: int32
    ///     /// doc comment for `b`
    ///     b: boolean
    ///     c: Vector2
    /// ) => (
    ///     int32
    ///     /// can also add docs here
    ///     isSuccess: boolean
    /// ) {
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
    pub fn eat_function(&mut self) -> ParseResult<NodeId<Function>> {
        // eat optional function keyword
        let style = if self.peek_keyword(Keyword::Function).is_ok() {
            self.bump();
            FunctionStyle::Function
        } else {
            FunctionStyle::Lambda
        };    
        // runtime (optional @)
        let runtime = if self.peek_token(TokenType::At).is_ok() {
            self.bump();
            FunctionRuntime::Static
        } else {
            FunctionRuntime::Dynamic
        };
        // name
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };
        // signature
        todo!()
    }

    /// Eat a function signature.
    ///
    /// Examples:
    /// ```
    /// (int32) => void
    /// () => int32
    /// (int32) => (int32, int32) // explicit tuple return type
    /// (int32) => int32, int32 // implicit tuple return type
    /// ```
    pub fn eat_function_signature(&mut self) -> ParseResult<NodeId<FunctionSignature>> {
        todo!()
    }
}
