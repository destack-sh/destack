use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::{
    AllocationMode, Attribute, BorrowRegion, CallBehavior, Function, Global, GlobalInitializer,
    Linkage, LocalNodeId, MemoryEffect, Mutability, PointerAttribute, Type, TypeAlias,
    TypeDeclarationSpans, TypeReference, Value, ValueReference,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl Parser {
    /// Parse one module.
    pub(super) fn parse_module(&mut self) {
        // forward declarations
        self.register_placeholders();

        // items
        while !self.peek_token(TokenType::End) {
            let recovery_pos = self.pos;
            if let Err(error) = self.parse_module_item() {
                self.diagnostics
                    .insert(error.to_diagnostic(self.content_id, self.file_id));
                self.try_recover_to_item(recovery_pos);
            }
        }
    }

    /// Parse one module item.
    fn parse_module_item(&mut self) -> ParseResult<()> {
        let item_start = self.pos();
        let (attributes, attribute_spans) = self.parse_attributes()?;

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

            self.parse_type_alias(item_start, attributes, attribute_spans)?;
        } else if self.peek_token(TokenType::Global) {
            self.parse_global(item_start, linkage, attributes, attribute_spans)?;
        } else if self.peek_token(TokenType::Function) {
            self.parse_function(item_start, linkage, attributes, attribute_spans)?;
        } else {
            return Err(ParseError::new(
                "expected 'type', 'function', or 'global'",
                self.pos(),
            ));
        }

        Ok(())
    }

    /// Recover to the next top level item boundary.
    fn try_recover_to_item(&mut self, recovery_pos: usize) {
        // make forward progress before scanning for the next item
        if self.pos == recovery_pos {
            self.bump();
        }

        while !self.peek_token(TokenType::End) {
            if self.peek_token(TokenType::At)
                || self.peek_token(TokenType::Extern)
                || self.peek_token(TokenType::Export)
                || self.peek_token(TokenType::Type)
                || self.peek_token(TokenType::Global)
                || self.peek_token(TokenType::Function)
            {
                return;
            }

            self.bump();
        }
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
                    let void_type = self.tree.insert_type(Type::Void);
                    let placeholder = Function {
                        name: name_id,
                        parameters: Vec::new(),
                        parameter_names: Vec::new(),
                        value_names: Vec::new(),
                        value_types: Vec::new(),
                        return_type: TypeReference::Type(void_type),
                        return_region: BorrowRegion::Inferred,
                        memory_effect: MemoryEffect::unknown(),
                        call_behavior: CallBehavior::unknown(),
                        allocation_size: None,
                        parameter_attributes: Vec::new(),
                        return_attribute: PointerAttribute::default(),
                        linkage: Linkage::Local,
                        allocation: AllocationMode::Any,
                        suspension: None,
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
        let parameters = self.scan_function_placeholder_parameters(linkage)?;

        // return type
        self.eat_token(TokenType::Colon)?;
        let return_type = self.parse_type()?;

        // seed the placeholder signature now so forward calls can resolve immediately
        let Some(function_id) = self.function_map.get(&name).copied() else {
            panic!("function placeholder missing for {name}");
        };
        let function = self.tree.get_mut(function_id);
        function.parameters = parameters;
        function.return_type = TypeReference::Type(return_type);

        // imports stop at the signature
        if linkage.is_import() {
            self.eat_token_maybe(TokenType::Semicolon);
            return Ok(());
        }

        // definitions: skip the body without trying to parse it yet
        self.skip_optional_braced_body();

        Ok(())
    }

    /// Scan one function parameter list for placeholder seeding.
    fn scan_function_placeholder_parameters(
        &mut self,
        linkage: Linkage,
    ) -> ParseResult<Vec<crate::Parameter>> {
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
                .into_iter()
                .enumerate()
                .map(|(index, ty)| crate::Parameter {
                    value: ValueReference::Value(crate::Value::new(index as u32)),
                    ty: TypeReference::Type(ty),
                })
                .collect()
        } else {
            let mut parameters = Vec::new();
            let mut next_value_id = 0u32;

            while !self.peek_token(TokenType::CloseParen) {
                let value = self.scan_function_placeholder_value(&mut next_value_id)?;
                self.eat_token(TokenType::Colon)?;
                let ty = self.parse_type()?;
                parameters.push(crate::Parameter {
                    value: ValueReference::Value(value),
                    ty: TypeReference::Type(ty),
                });
                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            parameters
        };

        self.eat_token(TokenType::CloseParen)?;

        Ok(parameters)
    }

    /// Scan one value definition for placeholder signature seeding.
    fn scan_function_placeholder_value(&mut self, next_value_id: &mut u32) -> ParseResult<Value> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value definition", self.pos()))?;
        let token_ty = token.ty;
        let token_text = self.tree.source_text(token.span).to_string();
        let token_start = token.start;

        match token_ty {
            TokenType::Value => {
                self.bump();

                let index: u32 = token_text
                    .strip_prefix('v')
                    .and_then(|text| text.parse().ok())
                    .ok_or_else(|| ParseError::invalid("value definition", token_start))?;
                *next_value_id = (*next_value_id).max(index + 1);
                Ok(Value::new(index))
            }
            TokenType::Identifier => {
                self.bump();

                let value = Value::new(*next_value_id);
                *next_value_id += 1;
                Ok(value)
            }
            _ => Err(ParseError::unexpected(
                "value definition",
                token_ty,
                token_start,
            )),
        }
    }

    /// Skip one braced body when present.
    fn skip_optional_braced_body(&mut self) {
        if !self.eat_token_maybe(TokenType::OpenBrace) {
            return;
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
    }

    /// Parse a type alias definition.
    pub(super) fn parse_type_alias(
        &mut self,
        item_start: usize,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<LocalNodeId<TypeAlias>> {
        // alias header
        let keyword_token = self.eat_token(TokenType::Type)?;
        let keyword_start = keyword_token.start;
        let keyword_length = self.tree.source_text(keyword_token.span).len();
        let keyword_span = self.span_at(keyword_start, keyword_length);

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
        let (ty, type_span, field_spans, declaration_spans) =
            if self.peek_token(TokenType::OpenBrace) {
                let type_start = self.pos();
                let (ty, field_spans, declaration_spans) = self.parse_struct_type()?;
                let type_span = self.span_from_parse_start(type_start);
                (ty, type_span, field_spans, declaration_spans)
            } else {
                let equals_token = self.eat_token(TokenType::Equals)?;
                let equals_start = equals_token.start;
                let equals_length = self.tree.source_text(equals_token.span).len();
                let equals_span = self.span_at(equals_start, equals_length);
                let (ty, type_span) = self.parse_type_part()?;
                (
                    ty,
                    type_span,
                    Vec::new(),
                    TypeDeclarationSpans::new(Some(equals_span), None, None),
                )
            };

        // record alias
        let name_id = self.strings.intern(&name);
        let alias = TypeAlias {
            name: name_id,
            ty: TypeReference::Type(placeholder_id),
        };
        let id = self.tree.insert(alias);
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));
        self.tree.set_keyword_span(id, keyword_span);
        self.tree.set_main_span(id, name_span);
        self.tree
            .set_side_span(id, NodeSpanType::Region(NodeSpanRegion::Type), type_span);
        self.tree
            .metadata
            .types
            .set_display_name(placeholder_id, name_id);
        self.tree.set_attribute_spans(id, attribute_spans);
        self.tree.set_type_field_spans(id, field_spans);
        self.tree.set_type_declaration_spans(id, declaration_spans);

        if ty != placeholder_id {
            let resolved = self.tree.get(ty).clone();
            *self.tree.get_mut(placeholder_id) = resolved;
            self.tree.metadata.copy_type_metadata(ty, placeholder_id);
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
    /// Expect `[export|extern] global name: type[, readonly][, space(name)] [ = init]`.
    pub(super) fn parse_global(
        &mut self,
        item_start: usize,
        linkage: Linkage,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<LocalNodeId<Global>> {
        // global header
        let keyword_token = self.eat_token(TokenType::Global)?;
        let keyword_start = keyword_token.start;
        let keyword_length = self.tree.source_text(keyword_token.span).len();
        let keyword_span = self.span_at(keyword_start, keyword_length);

        // global name
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = self.span_at(name_start, name.len());

        // type
        self.eat_token(TokenType::Colon)?;
        let (ty, type_span) = self.parse_type_part()?;

        // trailing qualifiers
        let mut mutability = Mutability::Mutable;
        let mut space = crate::AddressSpace::Local;
        while self.eat_token_maybe(TokenType::Comma) {
            if self.eat_token_maybe(TokenType::Readonly) {
                mutability = Mutability::Immutable;
                continue;
            }

            if self.eat_token_maybe(TokenType::AddressSpace) {
                self.eat_token(TokenType::OpenParen)?;
                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("global space", self.pos()))?;
                let text = self.tree.source_text(token.span).to_string();
                space = match token.ty {
                    TokenType::Identifier | TokenType::Local => {
                        crate::AddressSpace::from_name(&text)
                    }
                    _ => {
                        return Err(ParseError::unexpected(
                            "global space",
                            token.ty,
                            token.start,
                        ));
                    }
                };
                self.bump();
                self.eat_token(TokenType::CloseParen)?;
                continue;
            }

            return Err(ParseError::invalid("global qualifier", self.pos()));
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
            ty: TypeReference::Type(ty),
            mutability,
            space,
            linkage,
            initializer,
        };
        let id = self.tree.insert(global);
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));
        self.tree.set_keyword_span(id, keyword_span);
        self.tree.set_main_span(id, name_span);
        self.tree
            .set_side_span(id, NodeSpanType::Region(NodeSpanRegion::Type), type_span);
        self.tree.set_attribute_spans(id, attribute_spans);
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
            TokenType::Identifier if self.tree.source_text(token.span) == "zeroInit" => {
                self.bump();
                Ok(GlobalInitializer::Zero)
            }
            // byte string literal
            TokenType::Identifier
                if self.tree.source_text(token.span) == "b"
                    && self
                        .peek_nth_token(1)
                        .is_some_and(|token| token.ty == TokenType::StringLiteral) =>
            {
                self.eat_token(TokenType::Identifier)?;
                let token = self.eat_token(TokenType::StringLiteral)?;
                let token_text = self.tree.source_text(token.span).to_string();
                let token_start = token.start;
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // string literal
            TokenType::StringLiteral => {
                let token_text = self.tree.source_text(token.span).to_string();
                let token_start = token.start;
                self.bump();
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // scalar constant
            TokenType::Identifier if self.tree.source_text(token.span) == "null" => {
                let constant = self.parse_constant()?;
                Ok(GlobalInitializer::Scalar(constant))
            }
            TokenType::BooleanLiteral
            | TokenType::IntLiteral
            | TokenType::FloatLiteral
            | TokenType::CharacterLiteral => {
                let constant = self.parse_constant()?;
                Ok(GlobalInitializer::Scalar(constant))
            }
            // function address
            TokenType::Identifier if self.tree.source_text(token.span) == "functionAddress" => {
                self.bump();
                let (function, _span) = self.parse_function_reference_part()?;

                Ok(GlobalInitializer::FunctionAddress(function))
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
