use destack_core::Optional;
use destack_source::Span;

use crate::{
    ConstantId, ConstantRelocation, FunctionId, Global, GlobalId, GlobalLocation, Linkage,
    ParseError, ParseResult, Parser, Symbol, TokenType, TypeId,
};

/// Modifiers preceding one linkable declaration.
pub(super) struct DeclarationModifiers {
    /// The symbol linkage.
    linkage: Linkage,
    /// The global storage location when explicitly selected.
    location: Option<GlobalLocation>,
    /// Whether one global is immutable.
    is_readonly: bool,
}

impl DeclarationModifiers {
    /// Return the selected symbol linkage.
    pub(super) const fn linkage(&self) -> Linkage {
        self.linkage
    }
}

impl Parser<'_> {
    /// Parse one linkable function or global declaration.
    pub(super) fn parse_linkable_declaration(&mut self) -> ParseResult<()> {
        let modifiers = self.parse_declaration_modifiers();

        // dispatch the declaration after its modifiers are known
        if self.peek_name("function") {
            self.parse_function(modifiers.linkage)
        } else if self.peek_name("global") {
            let default_location = if modifiers.is_readonly {
                GlobalLocation::CONSTANT
            } else {
                GlobalLocation::LOCAL_STATIC
            };
            let location = match modifiers.location {
                Some(location) => location,
                None => default_location,
            };

            self.parse_global(modifiers.linkage, location, !modifiers.is_readonly)
        } else {
            let token = self.peek();

            Err(ParseError::new("expected function or global", token.span))
        }
    }

    /// Parse every modifier preceding one linkable declaration.
    pub(super) fn parse_declaration_modifiers(&mut self) -> DeclarationModifiers {
        let mut modifiers = DeclarationModifiers {
            linkage: Linkage::LOCAL,
            location: None,
            is_readonly: false,
        };

        // collect modifiers in source order
        loop {
            if self.eat_name_if("external") {
                modifiers.linkage = Linkage::EXTERNAL;
            } else if self.eat_name_if("export") {
                modifiers.linkage = Linkage::EXPORT;
            } else if self.eat_name_if("local") {
                modifiers.location = Some(GlobalLocation::LOCAL_STATIC);
            } else if self.eat_name_if("shared") {
                modifiers.location = Some(GlobalLocation::SHARED_STATIC);
            } else if self.eat_name_if("readonly") {
                modifiers.is_readonly = true;
            } else {
                break;
            }
        }

        modifiers
    }

    /// Parse one local function definition.
    pub(super) fn parse_function_declaration(&mut self) -> ParseResult<()> {
        self.parse_function(Linkage::LOCAL)
    }

    /// Parse one immutable byte sequence declaration.
    pub(super) fn parse_constant_declaration(&mut self) -> ParseResult<()> {
        self.eat_name("constant")?;
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();
        let id = self.constant_symbol(&text, name.span)?;

        // preserve indexed constant declaration order
        if id.index() != self.object.constant_count() {
            return Err(ParseError::new("duplicate constant", name.span));
        }

        let alignment_bytes = if self.eat_token_if(TokenType::Comma) {
            self.eat_name("align")?;
            self.eat_token(TokenType::OpenParenthesis)?;
            let alignment = self.parse_u32()?;
            self.eat_token(TokenType::CloseParenthesis)?;

            alignment
        } else {
            1
        };
        if alignment_bytes == 0 {
            return Err(ParseError::new(
                "constant alignment must be nonzero",
                name.span,
            ));
        }
        self.eat_token(TokenType::Equal)?;
        self.eat_name("bytes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let bytes = self.parse_bytes()?;

        let name = Optional::some(self.object.intern_string(&text));
        self.object.push_constant(name, alignment_bytes, bytes);

        Ok(())
    }

    /// Parse one global declaration or definition.
    fn parse_global(
        &mut self,
        linkage: Linkage,
        location: GlobalLocation,
        is_mutable: bool,
    ) -> ParseResult<()> {
        self.eat_name("global")?;
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();
        let id = self.global_symbol(&text, name.span)?;

        // preserve indexed global declaration order
        if id.index() != self.object.global_count() {
            return Err(ParseError::new("duplicate global", name.span));
        }
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_storage_type()?;

        let initializer = if self.eat_token_if(TokenType::Equal) {
            self.parse_global_initializer()?
        } else {
            Optional::none()
        };
        let name = self.object.intern_string(&text);
        let global = Global::new(name, linkage, location, is_mutable, ty, initializer);
        self.object.push_global(global);

        Ok(())
    }

    /// Parse one zero, constant, or function global initializer.
    fn parse_global_initializer(&mut self) -> ParseResult<Optional<ConstantId>> {
        // zero initialization has no constant payload
        if self.eat_name_if("zero") {
            return Ok(Optional::none());
        }

        // named constants retain their existing object identity
        if self.eat_name_if("constant") {
            let name = self.eat_token(TokenType::Identifier)?;
            let id = self.constant_symbol(self.text(name), name.span)?;

            return Ok(Optional::some(id));
        }

        self.eat_name("function")?;
        let name = self.eat_token(TokenType::Identifier)?;
        let function = self.function_symbol(self.text(name), name.span)?;
        let constant = self.object.push_constant(Optional::none(), 8, [0; 8]);
        let relocation = ConstantRelocation::new(0, Symbol::function(function.0), 0);
        self.object.push_constant_relocation(constant, relocation);

        Ok(Optional::some(constant))
    }

    /// Parse one comma-separated byte list.
    fn parse_bytes(&mut self) -> ParseResult<Vec<u8>> {
        let mut bytes = Vec::new();

        // parse bytes until the enclosing list closes
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            let token = self.eat_token(TokenType::Integer)?;
            let byte = self
                .text(token)
                .parse::<u8>()
                .map_err(|_| ParseError::new("expected byte", token.span))?;
            bytes.push(byte);

            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }

        Ok(bytes)
    }

    /// Resolve one predeclared constant symbol.
    fn constant_symbol(&self, name: &str, span: Span) -> ParseResult<ConstantId> {
        self.symbols
            .constants
            .get(name)
            .copied()
            .ok_or_else(|| ParseError::new("unknown constant", span))
    }

    /// Resolve one predeclared global symbol.
    fn global_symbol(&self, name: &str, span: Span) -> ParseResult<GlobalId> {
        self.symbols
            .globals
            .get(name)
            .copied()
            .ok_or_else(|| ParseError::new("unknown global", span))
    }

    /// Resolve one predeclared function symbol.
    pub(super) fn function_symbol(&self, name: &str, span: Span) -> ParseResult<FunctionId> {
        self.symbols
            .functions
            .get(name)
            .copied()
            .ok_or_else(|| ParseError::new("unknown function", span))
    }

    /// Parse one stored type symbol.
    pub(super) fn parse_storage_type(&mut self) -> ParseResult<TypeId> {
        let start = self.peek();

        // reuse one directly named runtime type
        if let Some(ty) = self.symbols.types.get(self.text(start)).copied() {
            self.bump();

            return Ok(ty);
        }

        self.parse_value_type()?;
        let end = self.previous();
        let text = &self.cursor.source[start.span.start as usize..end.span.end as usize];

        // reuse one previously materialized structural storage type
        if let Some(ty) = self.symbols.types.get(text).copied() {
            return Ok(ty);
        }

        let name = self.object.intern_string(text);
        let ty = self.object.push_type(name);
        self.symbols.types.insert(text.to_string(), ty);

        Ok(ty)
    }
}
