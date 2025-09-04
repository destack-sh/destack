use destack_language_token::TokenType;

use crate::{Function, Keyword, Let, NodeId, ParseResult, Parser, Trait};

impl<'a> Parser<'a> {
    /// Eat a Trait.
    ///
    /// Examples:
    /// ```
    /// trait { // anonymous trait
    ///     ...
    /// }
    ///
    /// trait Foo {
    ///     let x: int32 // constant
    ///     function foo() => int32
    /// }
    ///
    /// trait Baz<T> {
    ///     function baz() => T // semicolon optional
    /// }
    /// ```
    pub fn eat_trait(&mut self) -> ParseResult<NodeId<Trait>> {
        let start = self.mark();
        // header
        self.eat_keyword(Keyword::Trait)?;
        // name
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };
        // static parameters
        let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
            self.eat_token(TokenType::LessThan)?;
            let static_parameters = self.eat_parameters_body()?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(static_parameters)
        } else {
            None
        };
        // body (lets and functions)
        let mut lets: Vec<NodeId<Let>> = vec![];
        let mut functions: Vec<NodeId<Function>> = vec![];
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            self.eat_token(TokenType::OpenBrace)?;
            loop {
                if self.peek_keyword(Keyword::Let).is_ok() {
                    let let_id = self.eat_let_or_var()?;
                    lets.push(let_id);
                } else if self.peek_keyword(Keyword::Function).is_ok() {
                    let function_id = self.eat_function()?;
                    functions.push(function_id);
                } else if self.peek_token(TokenType::CloseBrace).is_ok() {
                    break;
                } else {
                    // TODO: report error
                    self.bump();
                    continue;
                }
            }
            self.eat_token(TokenType::CloseBrace)?;
        }
        let trait_id = self.tree.allocate(
            Trait {
                name,
                static_parameters,
                lets,
                functions,
            },
            self.get_span_from(start),
        );
        self.eat_token(TokenType::CloseBrace)?;
        Ok(trait_id)
    }
}
