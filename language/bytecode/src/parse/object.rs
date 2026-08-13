use crate::{Object, ParseError, ParseResult, Parser, TokenType};

impl Parser<'_> {
    /// Parse one complete bytecode object.
    pub fn parse(&mut self) -> ParseResult<Object> {
        while !self.peek_is(TokenType::End) {
            let token = self.peek();
            if !self.peek_name("external") && !self.peek_name("function") {
                return Err(ParseError::new("expected bytecode function", token.span));
            }

            self.parse_function()?;
        }

        // publish object-local function entries in symbol encounter order
        let functions = std::mem::take(&mut self.functions);
        let object = std::mem::take(&mut self.object)
            .functions(functions)
            .build();

        Ok(object)
    }
}
