use crate::source::TokenType;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, Copy, Function, Global, GlobalInitializer,
    Linkage, LocalNodeId, Mutability, Symbol, Type, TypeAlias, TypeDeclarationSpans, TypeId,
};

use super::error::{ParseError, ParseResult};
use super::function::FunctionHeaderMode;
use super::parser::Parser;

impl Parser {
    /// Parse one module.
    pub(super) fn parse_module(&mut self) {
        // forward declarations
        self.register_placeholders();

        // items
        while !self.peek_token(TokenType::End) {
            let recovery_pos = self.pos;
            let lifetime_scope_count = self.lifetime_scopes.len();
            if let Err(error) = self.parse_module_item() {
                self.restore_lifetime_scopes(lifetime_scope_count);
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

        // declaration modifiers
        let linkage = if self.peek_token(TokenType::External) {
            self.bump();
            Linkage::Import
        } else if self.peek_token(TokenType::Export) {
            self.bump();
            Linkage::Export
        } else {
            Linkage::Local
        };
        let mutability = if self.eat_token_maybe(TokenType::Readonly) {
            Mutability::Immutable
        } else {
            Mutability::Mutable
        };

        // item grammar
        if self.peek_token(TokenType::Type) {
            if linkage != Linkage::Local {
                return Err(ParseError::new(
                    "type aliases cannot be external or export",
                    self.pos(),
                ));
            }
            if mutability == Mutability::Immutable {
                return Err(ParseError::new(
                    "type aliases cannot be readonly",
                    self.pos(),
                ));
            }

            self.parse_type_alias(item_start, attributes, attribute_spans)?;
        } else if self.peek_token(TokenType::Global) {
            self.parse_global(item_start, linkage, mutability, attributes, attribute_spans)?;
        } else if self.peek_token(TokenType::Function) {
            if mutability == Mutability::Immutable {
                return Err(ParseError::new("functions cannot be readonly", self.pos()));
            }

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
                || self.peek_token(TokenType::External)
                || self.peek_token(TokenType::Export)
                || self.peek_token(TokenType::Readonly)
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
            if self.peek_token(TokenType::External) || self.peek_token(TokenType::Export) {
                self.bump();
            }

            // item mutability
            if self.peek_token(TokenType::Readonly) {
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
                    let placeholder =
                        Function::declare(name_id, Vec::new(), Vec::new(), TypeId::from(void_type));
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

            let linkage = if self.peek_token(TokenType::External) {
                self.bump();
                Linkage::Import
            } else if self.peek_token(TokenType::Export) {
                self.bump();
                Linkage::Export
            } else {
                Linkage::Local
            };
            if self.peek_token(TokenType::Readonly) {
                self.bump();
            }

            if self.peek_token(TokenType::Function) {
                let lifetime_scope_count = self.lifetime_scopes.len();
                let _ = self.seed_function_signature(linkage);
                self.restore_lifetime_scopes(lifetime_scope_count);
                continue;
            }

            self.bump();
        }

        self.pos = saved_pos;
        self.current_function = saved_function;
    }

    /// Skip attributes during the placeholder scan.
    fn skip_attribute_tokens(&mut self) {
        while self.peek_token(TokenType::At) {
            self.bump();
            let _ = self.eat_token_maybe(TokenType::Identifier);

            if self.peek_token(TokenType::OpenParenthesis) {
                self.bump();
                let mut depth = 1usize;
                while depth > 0 && !self.peek_token(TokenType::End) {
                    if self.peek_token(TokenType::OpenParenthesis) {
                        depth += 1;
                    } else if self.peek_token(TokenType::CloseParenthesis) {
                        depth = depth.saturating_sub(1);
                    }

                    self.bump();
                }
            }
        }
    }

    /// Seed one placeholder function signature.
    fn seed_function_signature(&mut self, linkage: Linkage) -> ParseResult<()> {
        let header = self.parse_function_header(linkage, FunctionHeaderMode::Placeholder)?;
        let function_id = header.function_id;
        let function = self.tree.get_mut(function_id);
        function.parameters = header.parameters;
        function.lifetimes = header.lifetimes;
        function.return_type = header.return_type;
        self.pop_lifetime_scope();

        // imports stop at the signature
        if linkage.is_import() {
            self.eat_token_maybe(TokenType::Semicolon);
            return Ok(());
        }

        // definitions: skip the body without trying to parse it yet
        self.skip_optional_braced_body();

        Ok(())
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
        let lifetimes = self.parse_lifetime_parameters()?;

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
                let equals_token = self.eat_token(TokenType::Equal)?;
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
            lifetimes: lifetimes.clone(),
            ty: TypeId::from(placeholder_id),
        };
        let id = self.tree.insert(alias);
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));
        self.tree.set_keyword_span(id, keyword_span);
        self.tree.set_main_span(id, name_span);
        self.tree
            .set_side_span(id, NodeSpanType::Region(NodeSpanRegion::Type), type_span);
        self.types.set_display_name(placeholder_id, name_id);
        self.tree.set_type_lifetimes(placeholder_id, lifetimes);
        self.tree.set_attribute_spans(id, attribute_spans);
        self.tree.set_type_field_spans(id, field_spans);
        self.tree.set_type_declaration_spans(id, declaration_spans);

        if ty != placeholder_id {
            let mut resolved = self.tree.get(ty).clone();
            if let Some(copy) = self.copy_attribute(&attributes, item_start)? {
                set_type_copy(&mut resolved, copy, item_start)?;
            }
            self.tree.set(placeholder_id, resolved);
            self.types.copy_type_entries(ty, placeholder_id);
            self.layouts.copy_type_entries(ty, placeholder_id);
            self.dispatch.copy_type_entries(ty, placeholder_id);
            self.drops.copy_type_entries(ty, placeholder_id);
        }
        self.type_alias_definitions.insert(name);
        self.pop_lifetime_scope();

        // optional declaration terminator
        self.eat_token_maybe(TokenType::Semicolon);

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Return the explicit copy attribute when present.
    fn copy_attribute(
        &self,
        attributes: &[Attribute],
        position: usize,
    ) -> ParseResult<Option<Copy>> {
        let mut copy = None;

        // find one explicit copy marker at most once
        for attribute in attributes {
            let AttributeIdentifier::Identifier(name) = attribute.name else {
                continue;
            };
            let name = self.strings.get(name);
            let next = match name {
                "copy" => Some(Copy::Yes),
                _ => None,
            };
            let Some(next) = next else {
                continue;
            };
            if copy.is_some() {
                return Err(ParseError::new("duplicate copy marker", position));
            }

            if !matches!(attribute.args, AttributeArgs::None) {
                return Err(ParseError::new(
                    "copy marker does not take arguments",
                    position,
                ));
            }

            copy = Some(next);
        }

        Ok(copy)
    }

    /// Parse a global definition or declaration.
    /// Expect `[export|external] [readonly] global name: type[, space(name)] [ = init]`.
    pub(super) fn parse_global(
        &mut self,
        item_start: usize,
        linkage: Linkage,
        mutability: Mutability,
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
        let colon_token = self.eat_token(TokenType::Colon)?;
        let (ty, type_span) = self.parse_type_use_after(colon_token, "global type");

        // trailing qualifiers
        let mut space = crate::Space::Local;
        while self.eat_token_maybe(TokenType::Comma) {
            if self.eat_token_maybe(TokenType::Space) {
                self.eat_token(TokenType::OpenParenthesis)?;
                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("global space", self.pos()))?;
                space = match self.token_type(token) {
                    TokenType::Identifier | TokenType::Local => {
                        let text = self.tree.source_text(token.span);
                        crate::Space::from_name(text).ok_or_else(|| {
                            ParseError::invalid_at_span(
                                "global space",
                                token.start,
                                token.span.end.saturating_sub(token.span.start) as usize,
                            )
                        })?
                    }
                    _ => {
                        return Err(ParseError::unexpected(
                            "global space",
                            self.token_type(token),
                            token.start,
                        ));
                    }
                };
                self.bump();
                self.eat_token(TokenType::CloseParenthesis)?;
                continue;
            }

            return Err(ParseError::invalid("global qualifier", self.pos()));
        }

        // initializer
        let initializer = if linkage.is_import() || !self.peek_token(TokenType::Equal) {
            None
        } else {
            self.eat_token(TokenType::Equal)?;
            Some(self.parse_data_init(Some(ty))?)
        };

        // record global
        let name_id = self.strings.intern(&name);
        let global = Global {
            name: name_id,
            symbol: Symbol(name_id),
            ty,
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
    fn parse_data_init(
        &mut self,
        expected_type: Option<LocalNodeId<Type>>,
    ) -> ParseResult<GlobalInitializer> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("data initializer", self.pos()))?;

        match self.token_type(token) {
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
                        .is_some_and(|token| self.token_type(token) == TokenType::String) =>
            {
                self.eat_token(TokenType::Identifier)?;
                let token = self.eat_token(TokenType::String)?;
                let token_text = self.tree.source_text(token.span).to_string();
                let token_start = token.start;
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // string literal
            TokenType::String => {
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
                let constant = if let Some(expected_type) = expected_type {
                    self.parse_constant_for_type(expected_type)?
                } else {
                    self.parse_constant()?
                };
                Ok(GlobalInitializer::Scalar(constant))
            }
            TokenType::BooleanLiteral
            | TokenType::Integer
            | TokenType::Float
            | TokenType::Character => {
                let constant = if let Some(expected_type) = expected_type {
                    self.parse_constant_for_type(expected_type)?
                } else {
                    self.parse_constant()?
                };
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
                    let element_type = self.data_init_element_type(expected_type, elements.len());
                    elements.push(self.parse_data_init(element_type)?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBrace)?;
                Ok(GlobalInitializer::Aggregate(elements))
            }
            _ => Err(ParseError::unexpected(
                "data initializer",
                self.token_type(token),
                token.start,
            )),
        }
    }

    /// Return the expected type for one aggregate initializer element.
    fn data_init_element_type(
        &self,
        expected_type: Option<LocalNodeId<Type>>,
        index: usize,
    ) -> Option<LocalNodeId<Type>> {
        let expected_type = expected_type?;

        match self.tree.get(expected_type) {
            Type::FixedArray { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. } => Some(*element),
            Type::Tuple { elements, .. } => elements.get(index).copied(),
            Type::Struct { fields, .. } => fields.get(index).map(|field| self.tree.get(*field).ty),
            Type::Newtype { inner, .. } => self.data_init_element_type(Some(*inner), index),
            _ => None,
        }
    }
}

/// Set the copy property on one explicit aggregate type.
fn set_type_copy(ty: &mut Type, copy: Copy, position: usize) -> ParseResult<()> {
    match ty {
        Type::FixedArray { copy: target, .. }
        | Type::Tuple { copy: target, .. }
        | Type::Struct { copy: target, .. }
        | Type::Newtype { copy: target, .. }
        | Type::Variant { copy: target, .. }
        | Type::Vector { copy: target, .. }
        | Type::Tensor { copy: target, .. } => {
            *target = copy;
            Ok(())
        }
        _ => Err(ParseError::new(
            "copy attribute requires an aggregate type",
            position,
        )),
    }
}
