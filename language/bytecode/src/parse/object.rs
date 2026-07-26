use crate::{Function, Object, ParseError, ParseResult, Parser, TokenType};

impl Parser<'_> {
    /// Parse one complete bytecode object.
    pub fn parse(&mut self) -> ParseResult<Object> {
        while !self.peek_is(TokenType::End) {
            let token = self.peek();
            if !self.peek_name("function") {
                return Err(ParseError::new("expected bytecode function", token.span));
            }

            self.parse_function()?;
        }

        // materialize referenced imports as empty physical rows
        let functions = std::mem::take(&mut self.functions)
            .into_iter()
            .map(|function| function.unwrap_or_else(Function::declaration));
        let object = std::mem::take(&mut self.object)
            .functions(functions)
            .build();

        Ok(object)
    }
}
