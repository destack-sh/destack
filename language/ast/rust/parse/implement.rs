use crate::parse::ParserOptions;
use crate::{BlockFormat, Implement, Keyword, NodeId, ParseResult, Parser};
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
                |parser| parser.eat_arguments_body(),
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

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        self.eat_newlines_maybe()?;
        let statements = self.eat_block_body(BlockFormat::Explicit)?;
        self.eat_token(TokenType::CloseBrace)?;

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

// todo! add tests for implement
