use crate::{Object, ParseError, ParseResult, Parser, TokenType};

impl Parser<'_> {
    /// Parse one complete bytecode object.
    pub fn parse(&mut self) -> ParseResult<Object> {
        while !self.peek_is(TokenType::End) {
            let token = self.peek();
            if !self.peek_name("function") && !self.peek_name("async") {
                return Err(ParseError::new("expected bytecode function", token.span));
            }

            self.parse_function()?;
        }

        // require every referenced function name to have one declaration
        let declarations = std::mem::take(&mut self.declarations)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| ParseError::new("function declaration is absent", self.empty_span()))?;
        let object = std::mem::take(&mut self.object)
            .functions(declarations)
            .build();

        Ok(object)
    }
}
