use crate::{
    AllocationMode, Attribute, CallBehavior, Function, Global, GlobalInitializer, Lifetime,
    Linkage, LocalNodeId, MemoryEffect, Mutability, PointerAttribute, Type, TypeAlias, TypedValue,
    Value,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Parse a module.
    pub(super) fn parse_module(&mut self) -> ParseResult<()> {
        // forward declarations
        self.register_placeholders();

        // items
        while !self.peek_token(TokenType::End) {
            let attributes = self.parse_attributes()?;

            // linkage
            let linkage = if self.peek_token(TokenType::Extern) {
                self.bump();
                Linkage::Import
            } else if self.peek_token(TokenType::Export) {
                self.bump();
                Linkage::Export
            } else {
                Linkage::Local
            };

            // item grammar
            if self.peek_token(TokenType::Type) {
                if linkage != Linkage::Local {
                    return Err(ParseError::new(
                        "type aliases cannot be extern or export",
                        self.pos(),
                    ));
                }

                self.parse_type_alias(attributes)?;
            } else if self.peek_token(TokenType::Global) {
                self.parse_global(linkage, attributes)?;
            } else if self.peek_token(TokenType::Function) {
                self.parse_function(linkage, attributes)?;
            } else {
                return Err(ParseError::new(
                    "expected 'type', 'function', or 'global'",
                    self.pos(),
                ));
            }
        }

        Ok(())
    }

    /// Pre register forward referenced item names.
    fn register_placeholders(&mut self) {
        let saved_pos = self.pos;
        let saved_function = self.current_function;

        // first pass: item placeholders
        while !self.peek_token(TokenType::End) {
            self.skip_attribute_tokens();

            // item linkage
            if self.peek_token(TokenType::Extern) || self.peek_token(TokenType::Export) {
                self.bump();
            }

            // function placeholders
            if self.peek_token(TokenType::Function) {
                self.bump();
                if let Some(name) = self.scan_symbol_name()
                    && !self.function_map.contains_key(&name)
                {
                    let name_id = self.strings.intern(&name);
                    let void_type = self.intern_type(Type::Void);
                    let placeholder = Function {
                        name: name_id,
                        parameters: Vec::new(),
                        parameter_names: Vec::new(),
                        value_types: Vec::new(),
                        return_type: void_type,
                        return_lifetime: Lifetime::Inferred,
                        memory_effect: MemoryEffect::unknown(),
                        call_behavior: CallBehavior::unknown(),
                        allocation_size: None,
                        parameter_attributes: Vec::new(),
                        return_attribute: PointerAttribute::default(),
                        linkage: Linkage::Local,
                        allocation: AllocationMode::Any,
                        suspension: None,
                        execution_model: None,
                        execution_stage: None,
                        workgroup_size: None,
                        environment: None,
                        locals: Vec::new(),
                        blocks: Vec::new(),
                        entry: None,
                        next_value_id: 0,
                    };
                    let function_id = self.tree.insert(placeholder);
                    self.function_map.insert(name, function_id);
                }

                continue;
            }

            // type placeholders
            if self.peek_token(TokenType::Type) {
                self.bump();
                if let Some(name) = self.scan_symbol_name()
                    && !self.type_alias_map.contains_key(&name)
                {
                    let type_id = self.tree.insert_type(Type::Void);
                    self.type_alias_map.insert(name, type_id);
                }

                continue;
            }

            // unrelated token
            self.bump();
        }

        // second pass: function signatures
        self.pos = 0;
        while !self.peek_token(TokenType::End) {
            self.skip_attribute_tokens();

            let linkage = if self.peek_token(TokenType::Extern) {
                self.bump();
                Linkage::Import
            } else if self.peek_token(TokenType::Export) {
                self.bump();
                Linkage::Export
            } else {
                Linkage::Local
            };

            if self.peek_token(TokenType::Function) {
                let _ = self.scan_function_placeholder_signature(linkage);
                continue;
            }

            self.bump();
        }

        self.pos = saved_pos;
        self.current_function = saved_function;
    }

    /// Skip attributes during the placeholder pre scan.
    fn skip_attribute_tokens(&mut self) {
        while self.peek_token(TokenType::At) {
            self.bump();
            let _ = self.eat_token_maybe(TokenType::Identifier);

            if self.peek_token(TokenType::OpenParen) {
                self.bump();
                let mut depth = 1usize;
                while depth > 0 && !self.peek_token(TokenType::End) {
                    if self.peek_token(TokenType::OpenParen) {
                        depth += 1;
                    } else if self.peek_token(TokenType::CloseParen) {
                        depth = depth.saturating_sub(1);
                    }

                    self.bump();
                }
            }
        }
    }

    /// Scan one function header and seed the placeholder signature.
    fn scan_function_placeholder_signature(&mut self, linkage: Linkage) -> ParseResult<()> {
        // function header
        self.eat_token(TokenType::Function)?;
        let (name, _) = self.parse_symbol_name()?;

        // parameter list
        self.eat_token(TokenType::OpenParen)?;
        let parameters = if linkage.is_import() {
            let mut parameter_types = Vec::new();
            while !self.peek_token(TokenType::CloseParen) {
                parameter_types.push(self.parse_type()?);
                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            parameter_types
                .iter()
                .enumerate()
                .map(|(index, &ty)| TypedValue {
                    value: Value::new(index as u32),
                    ty,
                })
                .collect::<Vec<_>>()
        } else {
            self.parse_typed_value_list()?
        };
        self.eat_token(TokenType::CloseParen)?;

        // return type
        self.eat_token(TokenType::Colon)?;
        let return_type = self.parse_type()?;

        // seed the placeholder signature now so forward calls can resolve immediately
        let Some(function_id) = self.function_map.get(&name).copied() else {
            panic!("function placeholder missing for {name}");
        };
        let function = self.tree.get_mut(function_id);
        function.parameters = parameters;
        function.return_type = return_type;

        // imports stop at the signature
        if linkage.is_import() {
            self.eat_token_maybe(TokenType::Semicolon);
            return Ok(());
        }

        // definitions: skip the body without trying to parse it yet
        if !self.eat_token_maybe(TokenType::OpenBrace) {
            return Ok(());
        }

        let mut depth = 1usize;
        while depth > 0 && !self.peek_token(TokenType::End) {
            if self.peek_token(TokenType::OpenBrace) {
                depth += 1;
            } else if self.peek_token(TokenType::CloseBrace) {
                depth = depth.saturating_sub(1);
            }

            self.bump();
        }

        Ok(())
    }

    /// Parse a type alias definition.
    pub(super) fn parse_type_alias(
        &mut self,
        attributes: Vec<Attribute>,
    ) -> ParseResult<LocalNodeId<TypeAlias>> {
        // alias header
        self.eat_token(TokenType::Type)?;

        // alias name
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = self.span_at(name_start, name.len());
        if self.type_alias_definitions.contains(&name) {
            return Err(ParseError::invalid(
                &format!("duplicate type alias '{name}'"),
                name_start,
            ));
        }

        // resolve placeholder
        let placeholder_id = match self.type_alias_map.get(&name).copied() {
            Some(existing) => existing,
            None => {
                let placeholder = self.tree.insert(Type::Void);
                self.type_alias_map.insert(name.clone(), placeholder);
                placeholder
            }
        };

        // alias target type
        if !self.peek_token(TokenType::OpenBrace) {
            self.eat_token(TokenType::Equals)?;
        }
        let ty = self.parse_type()?;

        // record alias
        let name_id = self.strings.intern(&name);
        let alias = TypeAlias {
            name: name_id,
            ty: placeholder_id,
        };
        let id = self.tree.insert(alias);
        self.tree.set_text_span(id, name_span);
        self.tree
            .metadata
            .layout
            .set_display_name(placeholder_id, name_id);

        if ty != placeholder_id {
            let resolved = self.tree.get(ty).clone();
            *self.tree.get_mut(placeholder_id) = resolved;
            self.tree
                .metadata
                .layout
                .copy_type_metadata(ty, placeholder_id);
            self.tree
                .metadata
                .dispatch
                .copy_type_metadata(ty, placeholder_id);
        }
        self.type_alias_definitions.insert(name);

        // optional declaration terminator
        self.eat_token_maybe(TokenType::Semicolon);

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse a global definition or declaration.
    /// Expect `[export|extern] global name: type[, readonly] [ = init]`.
    pub(super) fn parse_global(
        &mut self,
        linkage: Linkage,
        attributes: Vec<Attribute>,
    ) -> ParseResult<LocalNodeId<Global>> {
        // global header
        self.eat_token(TokenType::Global)?;

        // global name
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = self.span_at(name_start, name.len());

        // type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;

        // trailing mutability
        let mut mutability = Mutability::Mutable;
        if self.eat_token_maybe(TokenType::Comma) && self.eat_token_maybe(TokenType::Readonly) {
            mutability = Mutability::Immutable;
        }

        // initializer
        let initializer = if linkage.is_import() {
            None
        } else {
            self.eat_token(TokenType::Equals)?;
            Some(self.parse_data_init()?)
        };

        // record global
        let name_id = self.strings.intern(&name);
        let global = Global {
            name: name_id,
            ty,
            mutability,
            linkage,
            initializer,
        };
        let id = self.tree.insert(global);
        self.tree.set_text_span(id, name_span);
        self.global_map.insert(name, id);

        // optional declaration terminator
        self.eat_token_maybe(TokenType::Semicolon);

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse a data initializer.
    fn parse_data_init(&mut self) -> ParseResult<GlobalInitializer> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("data initializer", self.pos()))?;

        match token.ty {
            // zero initializer
            TokenType::Identifier if token.text == "zeroInit" => {
                self.bump();
                Ok(GlobalInitializer::Zero)
            }
            // byte string literal
            TokenType::Identifier
                if token.text == "b"
                    && self
                        .peek_nth_token(1)
                        .is_some_and(|token| token.ty == TokenType::StringLiteral) =>
            {
                self.eat_token(TokenType::Identifier)?;
                let token = self.eat_token(TokenType::StringLiteral)?;
                let token_text = token.text.to_string();
                let token_start = token.start;
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // string literal
            TokenType::StringLiteral => {
                let token_text = token.text.to_string();
                let token_start = token.start;
                self.bump();
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::String(value))
            }
            // scalar constant
            TokenType::Identifier if token.text == "null" => {
                let constant = self.parse_constant()?;
                Ok(GlobalInitializer::Scalar(constant))
            }
            TokenType::BoolLiteral
            | TokenType::IntLiteral
            | TokenType::FloatLiteral
            | TokenType::CharLiteral => {
                let constant = self.parse_constant()?;
                Ok(GlobalInitializer::Scalar(constant))
            }
            // aggregate initializer
            TokenType::OpenBrace => {
                self.bump();
                let mut elements = Vec::new();
                while !self.peek_token(TokenType::CloseBrace) {
                    elements.push(self.parse_data_init()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBrace)?;
                Ok(GlobalInitializer::Aggregate(elements))
            }
            _ => Err(ParseError::unexpected(
                "data initializer",
                token.ty,
                token.start,
            )),
        }
    }
}
