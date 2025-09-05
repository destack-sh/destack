//! Parse functions and closures.

use destack_language_token::TokenType;

use crate::{Function, FunctionRuntime, FunctionStyle, Keyword, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a Function or "lambda" definition or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    ///
    /// Examples:
    /// ```
    /// // function style
    ///
    /// function () // anonymous function with empty signature
    ///
    /// function foo() // just declaration, no body, no opening `{`
    ///
    /// function foo[T, U](x: T) => int32, boolean {
    ///    print("Hello, world!")
    /// }
    ///
    /// function baz(a: int32, b: boolean) => MyStruct, boolean {
    ///    ...
    /// }
    ///
    /// function @comptime() {
    ///    ...
    /// }
    ///
    /// function longBar(
    ///   /// doc comment for `a`
    ///   a: int32
    ///   /// doc comment for `b`
    ///   b: boolean
    ///   // regular comment
    ///   c: Vector2
    /// ) => int32, isGood: boolean {
    ///    ...
    /// )
    ///
    /// // lambda style
    ///
    /// function () { 0 }
    /// function x(x) { x + 1 }
    /// function y(x: int32) { x + 1 }
    /// ```
    pub fn eat_function(&mut self) -> ParseResult<NodeId<Function>> {
        let start = self.mark();

        // function
        self.eat_keyword(Keyword::Function)?;

        // runtime
        let runtime = if self.peek_token(TokenType::At).is_ok() {
            self.eat_token(TokenType::At)?;
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

        // static parameters
        let static_parameters = if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.eat_token(TokenType::OpenBracket)?;
            let static_parameters = self.eat_parameters_body()?;
            self.eat_token(TokenType::CloseBracket)?;
            Some(static_parameters)
        } else {
            None
        };

        // dynamic parameters
        self.eat_token(TokenType::OpenParenthesis)?;
        let dynamic_parameters = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            vec![]
        } else {
            self.eat_parameters_body()?
        };
        self.eat_token(TokenType::CloseParenthesis)?;

        // using
        let using = if self.peek_keyword(Keyword::Using).is_ok() {
            self.eat_keyword(Keyword::Union)?;
            let using = self.eat_using_header()?;
            Some(using)
        } else {
            None
        };

        // return type
        let return_type = if self.peek_token(TokenType::Arrow).is_ok() {
            self.eat_token(TokenType::Arrow)?;
            Some(self.eat_type()?)
        } else {
            None
        };

        // body
        let body = if self.peek_token(TokenType::OpenBrace).is_ok() {
            Some(self.eat_block()?)
        } else {
            None
        };

        let function_id = self.tree.allocate(
            Function {
                name,
                runtime,
                // TODO: support lambda function style
                style: FunctionStyle::Function,
                using,
                static_parameters,
                dynamic_parameters,
                return_type,
                body,
            },
            self.get_span_from(start),
        );
        Ok(function_id)
    }
}

#[cfg(test)]
mod tests {
    // todo!: test functions
}
