use crate::{Function, Implement, Keyword, Let, NodeId, ParseResult, Parser};
use dyst_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat an implement (incl. `implement` keyword).
    ///
    /// Examples:
    /// ```
    /// implement Foo {
    ///     ...
    /// }
    ///
    /// implement Foo<int32> {
    ///     ...
    /// }
    ///
    /// implement Marker for Bar; // optional semicolon
    /// implement OtherMarker for Bar
    ///
    /// implement Bar<int32> for Baz {
    ///     ...
    /// }
    ///
    /// implement<T> Bar<T> for Baz {
    ///     ...
    /// }
    /// ```
    pub fn eat_implement(&mut self) -> ParseResult<NodeId<Implement>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Implement)?;

        // static arguments
        let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
            self.eat_token(TokenType::LessThan)?;
            let static_arguments = self.eat_arguments_body()?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(static_arguments)
        } else {
            None
        };

        // target
        let receiver = self.eat_type()?;

        // for
        let for_trait = if self.peek_keyword(Keyword::For).is_ok() {
            self.eat_keyword(Keyword::For)?;
            Some(self.eat_type()?)
        } else {
            None
        };

        // body (if any)
        let mut lets: Vec<NodeId<Let>> = vec![];
        let mut functions: Vec<NodeId<Function>> = vec![];
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            self.eat_token(TokenType::OpenBrace)?;
            loop {
                if self.peek_keyword(Keyword::Let).is_ok()
                    || self.peek_keyword(Keyword::Var).is_ok()
                {
                    let let_id = self.eat_let_or_var()?;
                    lets.push(let_id);
                } else if self.peek_keyword(Keyword::Function).is_ok() {
                    let function_id = self.eat_function()?;
                    functions.push(function_id);
                } else if self.peek_token(TokenType::CloseBrace).is_ok() {
                    break;
                } else {
                    self.bump(); // :Diagnostic
                    continue;
                }
            }
            self.eat_token(TokenType::CloseBrace)?;
        } else {
            self.eat_any_stop_with_newlines()?;
        }

        // implement
        let implement_id = self.tree.allocate(
            Implement {
                static_arguments,
                receiver,
                for_trait,
                lets,
                functions,
            },
            self.get_span_from(start),
        );
        Ok(implement_id)
    }
}
