use crate::{Object, ParseError, ParseResult, Parser, TokenType};

impl Parser<'_> {
    /// Parse one complete bytecode object.
    pub fn parse(mut self) -> ParseResult<Object> {
        let declarations = self.declaration_positions()?;
        self.index_declarations(&declarations)?;
        self.collect_function_headers(&declarations)?;
        self.cursor.reset();

        // build every persisted object table in declaration order
        while !self.peek_is(TokenType::End) {
            let token = self.peek();

            match self.text(token) {
                "type" => self.parse_type_declaration()?,
                "constant" => self.parse_constant_declaration()?,
                "external" | "export" | "local" | "shared" | "readonly" | "global" => {
                    self.parse_linkable_declaration()?
                }
                "function" => self.parse_function_declaration()?,
                _ => return Err(ParseError::new("expected bytecode declaration", token.span)),
            }
        }

        Ok(self.object.build())
    }

    /// Collect every function header before function bodies are parsed.
    fn collect_function_headers(&mut self, declarations: &[usize]) -> ParseResult<()> {
        for position in declarations {
            let (linkage, keyword) = self.begin_declaration(*position)?;
            if self.text(keyword) == "function" {
                self.collect_function_header(linkage)?;
            }
        }

        self.cursor.reset();

        Ok(())
    }

    /// Parse one type symbol declaration.
    fn parse_type_declaration(&mut self) -> ParseResult<()> {
        self.eat_name("type")?;
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();

        if self.eat_token_if(TokenType::Equal) {
            return Err(ParseError::new(
                "bytecode types cannot be aliases",
                name.span,
            ));
        }

        // retain one opaque type symbol
        let Some(id) = self.symbols.types.get(&text).copied() else {
            return Err(ParseError::new("missing type predeclaration", name.span));
        };
        if id.index() >= self.object.type_count() {
            return Err(ParseError::new("missing indexed type", name.span));
        }

        Ok(())
    }
}
