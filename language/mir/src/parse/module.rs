use crate::source::TokenType;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, Copy, Function, Global, GlobalInitializer,
    Linkage, LocalNodeId, Mutability, Space, Symbol, Type, TypeDeclaration, TypeDeclarationSpans,
    TypeHeritage, TypeId,
};

use super::error::{ParseError, ParseResult};
use super::function::FunctionHeaderMode;
use super::parser::Parser;

impl Parser {
    /// Parse one module.
    pub(super) fn parse_module(&mut self) {
        // reserve declarations required by forward references
        self.reserve_items();

        // items
        while !self.peek_is(TokenType::End) {
            let recovery_pos = self.pos;
            let lifetime_scope_count = self.lifetime_scopes.len();
            if let Err(error) = self.parse_module_item() {
                self.restore_lifetime_scopes(lifetime_scope_count);
                self.diagnostics
                    .insert(error.to_diagnostic(self.blob, self.file_id));
                self.try_recover_to_item(recovery_pos);
            }
        }
    }

    /// Parse one module item.
    fn parse_module_item(&mut self) -> ParseResult<()> {
        let item_start = self.pos();
        let (attributes, attribute_spans) = self.parse_attributes()?;

        // declaration modifiers
        let linkage = if self.peek_is(TokenType::External) {
            self.bump();
            Linkage::Import
        } else if self.peek_is(TokenType::Export) {
            self.bump();
            Linkage::Export
        } else {
            Linkage::Local
        };
        let mutability = if self.eat_token_if(TokenType::Readonly) {
            Mutability::Immutable
        } else {
            Mutability::Mutable
        };
        let is_shared = self.eat_token_if(TokenType::Shared);

        // item grammar
        if self.peek_is(TokenType::Type) {
            if is_shared {
                return Err(ParseError::new("types cannot be shared", self.pos()));
            }
            if linkage != Linkage::Local {
                return Err(ParseError::new(
                    "type declarations cannot be external or export",
                    self.pos(),
                ));
            }
            if mutability == Mutability::Immutable {
                return Err(ParseError::new(
                    "type declarations cannot be readonly",
                    self.pos(),
                ));
            }

            self.parse_type_declaration(item_start, attributes, attribute_spans)?;
        } else if self.peek_is(TokenType::Constant) {
            if mutability == Mutability::Immutable {
                return Err(ParseError::new("constants are always readonly", self.pos()));
            }
            if is_shared {
                return Err(ParseError::new("constants cannot be shared", self.pos()));
            }

            self.parse_global(
                item_start,
                linkage,
                Mutability::Immutable,
                Space::Constant,
                TokenType::Constant,
                attributes,
                attribute_spans,
            )?;
        } else if self.peek_is(TokenType::Global) {
            let space = if is_shared {
                Space::Shared
            } else {
                Space::Local
            };
            self.parse_global(
                item_start,
                linkage,
                mutability,
                space,
                TokenType::Global,
                attributes,
                attribute_spans,
            )?;
        } else if self.peek_is(TokenType::Function) {
            if mutability == Mutability::Immutable {
                return Err(ParseError::new("functions cannot be readonly", self.pos()));
            }

            // raise a local function to shared linkage
            let linkage = match is_shared {
                true if linkage == Linkage::Local => Linkage::Shared,
                true => return Err(ParseError::new("shared functions link locally", self.pos())),
                false => linkage,
            };
            self.parse_function(item_start, linkage, attributes, attribute_spans)?;
        } else {
            return Err(ParseError::new(
                "expected 'type', 'function', 'global', or 'constant'",
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

        while !self.peek_is(TokenType::End) {
            if self.peek_is(TokenType::At)
                || self.peek_is(TokenType::External)
                || self.peek_is(TokenType::Export)
                || self.peek_is(TokenType::Readonly)
                || self.peek_is(TokenType::Shared)
                || self.peek_is(TokenType::Type)
                || self.peek_is(TokenType::Global)
                || self.peek_is(TokenType::Constant)
                || self.peek_is(TokenType::Function)
            {
                return;
            }

            self.bump();
        }
    }

    /// Reserve declarations required to parse forward references.
    fn reserve_items(&mut self) {
        let saved_pos = self.pos;
        let saved_function = self.current_function;

        // first pass: types required by concrete function identities
        self.reserve_types();

        // second pass: concrete functions
        self.pos = 0;
        self.reserve_functions();

        // third pass: function signatures
        self.pos = 0;
        while !self.peek_is(TokenType::End) {
            self.skip_attribute_tokens();

            let linkage = if self.peek_is(TokenType::External) {
                self.bump();
                Linkage::Import
            } else if self.peek_is(TokenType::Export) {
                self.bump();
                Linkage::Export
            } else {
                Linkage::Local
            };
            if self.peek_is(TokenType::Readonly) {
                self.bump();
            }
            if self.peek_is(TokenType::Shared) {
                self.bump();
            }

            if self.peek_is(TokenType::Function) {
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

    /// Reserve identified types before parsing concrete function arguments.
    fn reserve_types(&mut self) {
        // reserve each name without inspecting its definition
        while !self.peek_is(TokenType::End) {
            self.skip_attribute_tokens();
            if self.peek_is(TokenType::Type) {
                self.bump();
                let Ok((name, _)) = self.parse_symbol_name() else {
                    continue;
                };
                if !self.type_declaration_map.contains_key(&name) {
                    let name_id = self.strings.intern(&name);
                    let symbol = Symbol::named(self.module, name_id);
                    let type_id = self.tree.reserve_type(symbol);
                    self.type_declaration_map.insert(name, type_id);
                }
            } else {
                self.bump();
            }
        }
    }

    /// Reserve concrete functions after every identified type is available.
    fn reserve_functions(&mut self) {
        while !self.peek_is(TokenType::End) {
            self.skip_attribute_tokens();

            // skip declaration modifiers
            if self.peek_is(TokenType::External) || self.peek_is(TokenType::Export) {
                self.bump();
            }
            if self.peek_is(TokenType::Readonly) {
                self.bump();
            }
            if self.peek_is(TokenType::Shared) {
                self.bump();
            }
            if self.peek_is(TokenType::Function) {
                self.bump();

                let lifetime_scope_count = self.lifetime_scopes.len();
                let parsed = self.parse_symbol_name().and_then(|(name, _)| {
                    let (arguments, _, _) = self.parse_declaration_parameters(false)?;

                    Ok((name, arguments))
                });
                self.restore_lifetime_scopes(lifetime_scope_count);
                let Ok((name, arguments)) = parsed else {
                    continue;
                };

                let key = (name.clone(), arguments.clone());
                if self.function_map.contains_key(&key) {
                    continue;
                }

                let name_id = self.strings.intern(&name);
                let void_type = self.tree.intern_type(Type::Void);
                let base = Symbol::named(self.module, name_id);
                let symbol = base.instantiate(&arguments, &self.tree);
                let function = Function::declare(
                    self.module,
                    name_id,
                    Vec::new(),
                    Vec::new(),
                    TypeId::from(void_type),
                )
                .with_arguments(arguments)
                .with_symbol(symbol);
                let function_id = self.tree.insert(function);
                self.function_map.insert(key, function_id);

                continue;
            }

            self.bump();
        }

        // link each specialization to the template its name and arguments apply
        let specializations: Vec<_> = self
            .function_map
            .iter()
            .filter(|((_, arguments), _)| !arguments.is_empty())
            .map(|((name, _), id)| (name.clone(), *id))
            .collect();
        for (name, id) in specializations {
            let template = self.function_map.get(&(name, Vec::new())).copied();
            self.tree.get_mut(id).template = template;
        }
    }

    /// Skip attributes while reserving declarations.
    fn skip_attribute_tokens(&mut self) {
        while self.peek_is(TokenType::At) {
            self.bump();
            let _ = self.eat_token_if(TokenType::Identifier);

            if self.peek_is(TokenType::OpenParenthesis) {
                self.bump();
                let mut depth = 1usize;
                while depth > 0 && !self.peek_is(TokenType::End) {
                    if self.peek_is(TokenType::OpenParenthesis) {
                        depth += 1;
                    } else if self.peek_is(TokenType::CloseParenthesis) {
                        depth -= 1;
                    }

                    self.bump();
                }
            }
        }
    }

    /// Seed one reserved function signature.
    fn seed_function_signature(&mut self, linkage: Linkage) -> ParseResult<()> {
        let header = self.parse_function_header(linkage, FunctionHeaderMode::Signature)?;
        let function_id = header.function_id;
        let function = self.tree.get_mut(function_id);
        function.arguments = header.arguments;
        function.generics = header.generics.clone();
        function.parameters = header.parameters;
        function.lifetimes = header.lifetimes;
        function.return_type = header.return_type;
        self.pop_lifetime_scope();

        // imports stop at the signature
        if linkage.is_import() {
            self.eat_token_if(TokenType::Semicolon);
            return Ok(());
        }

        // definitions: skip the body without trying to parse it yet
        self.skip_optional_braced_body();

        Ok(())
    }

    /// Skip one braced body when present.
    fn skip_optional_braced_body(&mut self) {
        if !self.eat_token_if(TokenType::OpenBrace) {
            return;
        }

        let mut depth = 1usize;
        while depth > 0 && !self.peek_is(TokenType::End) {
            if self.peek_is(TokenType::OpenBrace) {
                depth += 1;
            } else if self.peek_is(TokenType::CloseBrace) {
                depth -= 1;
            }

            self.bump();
        }
    }

    /// Parse a type declaration definition.
    pub(super) fn parse_type_declaration(
        &mut self,
        item_start: usize,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<LocalNodeId<TypeDeclaration>> {
        // declaration header
        let keyword_token = self.eat_token(TokenType::Type)?;
        let keyword_start = keyword_token.start();
        let keyword_length = self.tree.source_text(keyword_token.span).len();
        let keyword_span = self.span_at(keyword_start, keyword_length);

        // declaration name
        let (name, name_start) = self.parse_symbol_name()?;
        let (arguments, generics, _) = self.parse_declaration_parameters(true)?;
        let name_span = if arguments.is_empty() && generics.is_empty() {
            self.span_at(name_start, name.len())
        } else {
            self.span_between(name_start, self.pos())
        };

        // reject arguments on a declaration and names already declared
        if !arguments.is_empty() {
            return Err(ParseError::invalid(
                "type declaration arguments",
                name_start,
            ));
        }
        if self.type_declaration_definitions.contains(&name) {
            return Err(ParseError::invalid(
                &format!("duplicate type declaration '{name}'"),
                name_start,
            ));
        }

        // resolve the reserved type identity
        let type_id = match self.type_declaration_map.get(&name).copied() {
            Some(existing) => existing,
            None => {
                let symbol = Symbol::named(self.module, self.strings.intern(&name));
                let reserved = self.tree.reserve_type(symbol);
                self.type_declaration_map.insert(name.clone(), reserved);
                reserved
            }
        };

        // direct nominal heritage
        let heritage = self.parse_type_heritage()?;

        // record an opaque declaration and stop before a definition
        if self.eat_token_if(TokenType::Semicolon) {
            let name_id = self.strings.intern(&name);
            let id = self
                .tree
                .insert_type_declaration(name_id, generics, type_id, heritage);
            self.tree
                .set_text_span(id, self.span_from_parse_start(item_start));
            self.tree.set_keyword_span(id, keyword_span);
            self.tree.set_main_span(id, name_span);
            self.tree.set_attribute_spans(id, attribute_spans);
            self.type_declaration_definitions.insert(name.clone());
            self.pop_lifetime_scope();

            return Ok(id);
        }

        // declaration target type
        let (ty, type_span, field_spans, declaration_spans) = if self.peek_is(TokenType::OpenBrace)
        {
            let type_start = self.pos();
            let (ty, field_spans, declaration_spans) = self.parse_struct_type()?;
            let type_span = self.span_from_parse_start(type_start);
            (ty, type_span, field_spans, declaration_spans)
        } else {
            let equals_token = self.eat_token(TokenType::Equal)?;
            let equals_start = equals_token.start();
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

        // reject direct self definitions
        if ty == type_id {
            let length = type_span.end.saturating_sub(type_span.start) as usize;
            return Err(ParseError::with_length(
                "type declaration cannot define itself",
                type_span.start as usize,
                length,
            ));
        }

        // reject duplicate definitions of one declared name
        if self.tree.is_defined_type(type_id) {
            return Err(ParseError::new(
                format!("type '{name}' is already defined"),
                item_start,
            ));
        }

        // define the identified representation
        let mut resolved = self.tree.get(ty).clone();
        if let Some(copy) = self.copy_attribute(&attributes, item_start)? {
            set_type_copy(&mut resolved, copy, item_start)?;
        }
        self.tree.define_type(type_id, resolved);
        self.layouts.copy_type_entries(ty, type_id);
        self.dispatch.copy_type_entries(ty, type_id);
        self.drops.copy_type_entries(ty, type_id);

        // record declaration
        let name_id = self.strings.intern(&name);
        let id = self
            .tree
            .insert_type_declaration(name_id, generics, type_id, heritage);
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));
        self.tree.set_keyword_span(id, keyword_span);
        self.tree.set_main_span(id, name_span);
        self.tree
            .set_side_span(id, NodeSpanType::Region(NodeSpanRegion::Type), type_span);
        self.tree.set_attribute_spans(id, attribute_spans);
        self.tree.set_type_field_spans(id, field_spans);
        self.tree.set_type_declaration_spans(id, declaration_spans);

        self.type_declaration_definitions.insert(name.clone());
        self.pop_lifetime_scope();

        // optional declaration terminator
        self.eat_token_if(TokenType::Semicolon);

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse direct nominal heritage after one declared type name.
    fn parse_type_heritage(&mut self) -> ParseResult<TypeHeritage> {
        // inherited types
        let mut extends = Vec::new();
        if self.eat_name_if("extends") {
            loop {
                extends.push(self.parse_type_part()?.0);
                if !self.eat_token_if(TokenType::Comma) {
                    break;
                }
            }
        }

        // implemented interfaces
        let mut implements = Vec::new();
        if self.eat_name_if("implements") {
            loop {
                implements.push(self.parse_type_part()?.0);
                if !self.eat_token_if(TokenType::Comma) {
                    break;
                }
            }
        }

        Ok(TypeHeritage {
            extends,
            implements,
        })
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

    /// Parse a global or constant definition or declaration.
    pub(super) fn parse_global(
        &mut self,
        item_start: usize,
        linkage: Linkage,
        mutability: Mutability,
        space: Space,
        keyword: TokenType,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<LocalNodeId<Global>> {
        // global header
        let keyword_token = self.eat_token(keyword)?;
        let keyword_start = keyword_token.start();
        let keyword_length = self.tree.source_text(keyword_token.span).len();
        let keyword_span = self.span_at(keyword_start, keyword_length);

        // global name
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = self.span_at(name_start, name.len());

        // type
        let colon_token = self.eat_token(TokenType::Colon)?;
        let (ty, type_span) = self.parse_type_use_after(colon_token, "global type");

        // initializer
        let initializer = if linkage.is_import() || !self.peek_is(TokenType::Equal) {
            None
        } else {
            self.eat_token(TokenType::Equal)?;
            Some(self.parse_data_init(Some(ty))?)
        };

        // record global
        let name_id = self.strings.intern(&name);
        let global = Global {
            name: name_id,
            symbol: Symbol::named(self.module, name_id),
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
        self.eat_token_if(TokenType::Semicolon);

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
            TokenType::Identifier if self.tree.source_text(token.span) == "zeroinit" => {
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
                let token_start = token.start();
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // string literal: a value form for nominal representations, raw bytes otherwise
            TokenType::String => {
                let token_text = self.tree.source_text(token.span).to_string();
                let token_start = token.start();
                self.bump();
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                let is_nominal = expected_type
                    .is_some_and(|ty| matches!(self.tree.type_definition(ty), Type::Struct { .. }));
                if is_nominal {
                    let value = self.strings.intern(&value);

                    return Ok(GlobalInitializer::String(value));
                }

                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // bigint literal like 100n or -100n
            TokenType::Integer
                if self
                    .tree
                    .source_text(token.span)
                    .strip_suffix('n')
                    .is_some_and(|digits| {
                        let digits = digits.strip_prefix('-').unwrap_or(digits);

                        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
                    }) =>
            {
                let token_text = self.tree.source_text(token.span).to_string();
                let token_start = token.start();
                self.bump();
                let digits = token_text.strip_suffix('n').unwrap_or(&token_text);
                let value = digits.parse::<i64>().map_err(|_| {
                    ParseError::invalid(&format!("bigint literal '{token_text}'"), token_start)
                })?;

                Ok(GlobalInitializer::BigInt(value))
            }
            // scalar constant
            TokenType::Identifier
                if matches!(self.tree.source_text(token.span), "null" | "undefined") =>
            {
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
                let (function, arguments, span) = self.parse_function_reference_part()?;
                if !arguments.is_empty() {
                    return Err(ParseError::invalid(
                        "template reference outside a call",
                        span.start as usize,
                    ));
                }

                Ok(GlobalInitializer::FunctionAddress(function))
            }
            // global address
            TokenType::Identifier if self.tree.source_text(token.span) == "globalAddress" => {
                self.bump();
                let (global, _span) = self.parse_global_reference_part()?;

                Ok(GlobalInitializer::GlobalAddress(global))
            }
            // aggregate initializer
            TokenType::OpenBrace => {
                self.bump();
                let mut elements = Vec::new();
                while !self.peek_is(TokenType::CloseBrace) {
                    let element_type = self.data_init_element_type(expected_type, elements.len());
                    elements.push(self.parse_data_init(element_type)?);
                    if !self.eat_token_if(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBrace)?;
                Ok(GlobalInitializer::Aggregate(elements))
            }
            _ => Err(ParseError::unexpected(
                "data initializer",
                self.token_type(token),
                token.start(),
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

        match self.tree.type_definition(expected_type) {
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => Some(*element),
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
        Type::Struct { copy: target, .. }
        | Type::Newtype { copy: target, .. }
        | Type::Variant { copy: target, .. } => {
            *target = copy;
            Ok(())
        }
        _ => Err(ParseError::new(
            "copy attribute requires an aggregate type",
            position,
        )),
    }
}
