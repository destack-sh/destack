use destack_core::Optional;
use destack_source::Span;

use crate::{FunctionTypeId, Object, ParseError, ParseResult, Parser, TokenType};

use super::symbol::FunctionTypeDefinition;

impl Parser<'_> {
    /// Parse one complete bytecode object.
    pub fn parse(mut self) -> ParseResult<Object> {
        let declarations = self.declaration_positions()?;
        self.index_declarations(&declarations)?;
        self.collect_function_types(&declarations)?;
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

    /// Collect every function type before function bodies are parsed.
    fn collect_function_types(&mut self, declarations: &[usize]) -> ParseResult<()> {
        // build every named function type first
        for position in declarations {
            let (_, keyword) = self.begin_declaration(*position)?;
            match self.text(keyword) {
                "type" if self.peek_is(TokenType::Identifier) => {
                    let position = self.cursor.position();
                    self.bump();
                    if self.peek_is(TokenType::Equal) {
                        self.cursor.seek(position);
                        self.collect_named_function_type()?;
                    }
                }
                _ => {}
            }
        }

        // collect function-owned types after the named prefix
        for position in declarations {
            let (linkage, keyword) = self.begin_declaration(*position)?;
            if self.text(keyword) == "function" {
                self.collect_function_type(linkage)?;
            }
        }

        self.cursor.reset();

        Ok(())
    }

    /// Collect one named function type.
    fn collect_named_function_type(&mut self) -> ParseResult<()> {
        let name = self.eat_token(TokenType::Identifier)?;
        let span = name.span;
        let text = self.text(name).to_string();
        let function_type = self
            .symbols
            .function_types
            .get(&text)
            .copied()
            .ok_or_else(|| ParseError::new("missing indexed function type", name.span))?;
        self.eat_token(TokenType::Equal)?;
        let definition = self.parse_function_type_definition()?;
        self.insert_function_type(function_type, definition.clone(), span)?;

        let name = Optional::some(self.object.intern_string(&text));
        let appended =
            self.object
                .push_function_type(name, definition.parameters, definition.results);
        if appended != function_type {
            return Err(ParseError::new("function types are not dense", span));
        }

        Ok(())
    }

    /// Store one indexed function type definition.
    pub(super) fn insert_function_type(
        &mut self,
        function_type: FunctionTypeId,
        definition: FunctionTypeDefinition,
        span: Span,
    ) -> ParseResult<()> {
        if function_type.index() != self.symbols.function_type_definitions.len() {
            return Err(ParseError::new("function types are not dense", span));
        }
        self.symbols.function_type_definitions.push(definition);

        Ok(())
    }

    /// Parse one type symbol declaration.
    fn parse_type_declaration(&mut self) -> ParseResult<()> {
        self.eat_name("type")?;
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();

        // parse a named function type
        if self.eat_token_if(TokenType::Equal) {
            return self.parse_function_type_declaration(text, name.span);
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

    /// Parse one named function type declaration.
    fn parse_function_type_declaration(&mut self, text: String, span: Span) -> ParseResult<()> {
        let Some(id) = self.symbols.function_types.get(&text).copied() else {
            return Err(ParseError::new(
                "missing function type predeclaration",
                span,
            ));
        };
        let parsed = self.parse_function_type_definition()?;
        let expected = &self.symbols.function_type_definitions[id.index()];
        if parsed != *expected {
            return Err(ParseError::new(
                "function type changed after indexing",
                span,
            ));
        }

        if self.object.function_type(id).is_none() {
            return Err(ParseError::new("missing indexed function type", span));
        }
        self.eat_token_if(TokenType::Semicolon);

        Ok(())
    }

    /// Parse one function type definition.
    pub(super) fn parse_function_type_definition(&mut self) -> ParseResult<FunctionTypeDefinition> {
        let parameters = self.parse_value_types()?;
        self.eat_token(TokenType::FatArrow)?;
        let results = if self.peek_is(TokenType::OpenParenthesis) {
            self.parse_value_types()?
        } else if self.eat_name_if("void") {
            Vec::new()
        } else {
            let result = self.parse_value_type()?;
            vec![result]
        };

        Ok(FunctionTypeDefinition {
            parameters,
            results,
        })
    }
}
