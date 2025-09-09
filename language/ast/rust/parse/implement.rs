use crate::parse::ParserOptions;
use crate::{Implement, Keyword, NodeId, ParseResult, Parser, Statement};
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
            let static_arguments = self.with_options(
                ParserOptions {
                    in_static_type: true,
                    ..self.options
                },
                |p| p.eat_arguments_body(),
            )?;
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
        let mut statements: Vec<NodeId<Statement>> = vec![];
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            self.eat_token(TokenType::OpenBrace)?;
            loop {
                // stop on closing brace
                if self.peek_token(TokenType::CloseBrace).is_ok() {
                    break;
                }
                // consume any stop
                else if self.peek_any_stop().is_ok() {
                    self.eat_any_stop_with_newlines()?;
                }
                // eat statements
                else {
                    let statement_id = self.eat_statement()?;
                    statements.push(statement_id);
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
                statements,
            },
            self.get_span_from(start),
        );
        Ok(implement_id)
    }
}
