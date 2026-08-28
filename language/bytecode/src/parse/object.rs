use destack_source::ProvenanceBuilder;

use crate::{Object, ParseError, ParseResult, Parser, TokenType};

impl Parser<'_> {
    /// Parse one bytecode object and append its authored provenance.
    pub fn parse(&mut self, provenance: &mut ProvenanceBuilder) -> ParseResult<Object> {
        let mark = provenance.mark();

        // parse each physical function transactionally
        while !self.peek_is(TokenType::End) {
            let token = self.peek();
            let result = if self.peek_name("external") || self.peek_name("function") {
                self.parse_function(provenance)
            } else {
                Err(ParseError::new("expected bytecode function", token.span))
            };
            if let Err(error) = result {
                provenance.restore(mark);

                return Err(error);
            }
        }

        // publish object-local function entries in symbol encounter order
        let functions = std::mem::take(&mut self.functions);
        let object = std::mem::take(&mut self.object)
            .functions(functions)
            .build();

        Ok(object)
    }
}
