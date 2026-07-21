use crate::source::{Token, TokenType};
use destack_source::Span;

use crate::{
    Access, BorrowObligation, Copy, Field, FieldSpan, Lifetime, LifetimeParameter, LifetimeTerm,
    LocalNodeId, Nullability, ReferenceKind, SignatureParameter, Space, TensorDimension,
    TensorDimensionOrder, TensorFormat, TensorReduction, TensorSharding, TensorShardingAxis,
    TensorViewFormat, Type, TypeDeclarationSpans, TypeId, VariantCase,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

/// Parsed reference-like type qualifiers.
#[derive(Debug, Clone)]
struct ReferenceQualifiers {
    /// The reference ownership kind.
    kind: Option<ReferenceKind>,
    /// The explicit reference lifetime.
    lifetime: Lifetime,
    /// The referenced space.
    space: Space,
    /// The exposed access mode.
    access: Option<Access>,
    /// The accepted nullish values.
    nullability: Nullability,
}

impl ReferenceQualifiers {
    /// Create empty reference qualifiers.
    fn new(nullability: Nullability) -> Self {
        Self {
            kind: None,
            lifetime: Lifetime::empty(),
            space: Space::Local,
            access: None,
            nullability,
        }
    }

    /// Finish qualifiers that require an ownership kind.
    fn finish(
        self,
        expected: &'static str,
        pos: usize,
    ) -> ParseResult<ResolvedReferenceQualifiers> {
        let kind = self
            .kind
            .ok_or_else(|| ParseError::invalid(expected, pos))?;

        if matches!(kind, ReferenceKind::Unique | ReferenceKind::Raw) && !self.lifetime.is_empty() {
            return Err(ParseError::invalid("reference lifetime", pos));
        }

        Ok(ResolvedReferenceQualifiers {
            kind,
            lifetime: self.lifetime,
            space: self.space,
            access: self
                .access
                .ok_or_else(|| ParseError::invalid("reference access", pos))?,
            nullability: self.nullability,
        })
    }
}

/// Resolved reference-like type qualifiers.
#[derive(Debug, Clone)]
struct ResolvedReferenceQualifiers {
    /// The reference ownership kind.
    kind: ReferenceKind,
    /// The explicit reference lifetime.
    lifetime: Lifetime,
    /// The referenced space.
    space: Space,
    /// The exposed access mode.
    access: Access,
    /// The accepted nullish values.
    nullability: Nullability,
}

impl Parser {
    /// Parse a type expression and return its enclosing span.
    pub(super) fn parse_type_part(&mut self) -> ParseResult<(LocalNodeId<Type>, Span)> {
        let type_start = self.pos();
        let ty = self.parse_type()?;
        let span = self.span_from_parse_start(type_start);

        Ok((ty, span))
    }

    /// Parse a type use and return its enclosing span.
    pub(super) fn parse_type_use_part(&mut self) -> ParseResult<(TypeId, Span)> {
        let type_start = self.pos();
        let ty = self.parse_type()?;
        let lifetimes = self.parse_type_lifetime_arguments()?;
        let ty = self.apply_type_lifetimes(ty, lifetimes)?;
        let span = self.span_from_parse_start(type_start);

        Ok((ty, span))
    }

    /// Parse a type use after one required delimiter.
    pub(super) fn parse_type_use_after(
        &mut self,
        delimiter: Token,
        expected: &'static str,
    ) -> (TypeId, Span) {
        self.parse_type_use_recovering(delimiter, expected, true)
    }

    /// Parse a return type use after its required delimiter.
    pub(super) fn parse_return_type_use_after(&mut self, delimiter: Token) -> (TypeId, Span) {
        let is_structural_type_allowed = self.is_return_structural_type_start();

        self.parse_type_use_recovering(delimiter, "return type", is_structural_type_allowed)
    }

    /// Parse a type use after one required delimiter and recover holes locally.
    fn parse_type_use_recovering(
        &mut self,
        delimiter: Token,
        expected: &'static str,
        allow_structural_type: bool,
    ) -> (TypeId, Span) {
        let hole_position = delimiter.span.end as usize;

        if self.is_missing_type_position(&delimiter, allow_structural_type) {
            let error = ParseError::new(format!("expected {expected}"), hole_position);

            return self.recovered_type(error, false);
        }

        let type_start = self.pos();
        match self.parse_type() {
            Ok(ty) => match self.parse_type_lifetime_arguments() {
                Ok(lifetimes) => {
                    let ty = self
                        .apply_type_lifetimes(ty, lifetimes)
                        .unwrap_or_else(|error| {
                            self.diagnostics
                                .insert(error.to_diagnostic(self.content_id, self.file_id));
                            self.error_type()
                        });
                    let span = self.span_from_parse_start(type_start);

                    (ty, span)
                }
                Err(error) => self.recovered_type(error, true),
            },
            Err(error) => self.recovered_type(error, true),
        }
    }

    /// Emit one type recovery diagnostic and return the canonical error type.
    fn recovered_type(&mut self, error: ParseError, should_advance: bool) -> (TypeId, Span) {
        self.diagnostics
            .insert(error.to_diagnostic(self.content_id, self.file_id));

        if should_advance && self.peek().is_some() {
            self.bump();
        }

        let error_end = self.pos();
        let span = self.span_between(error.position(), error_end);
        let ty = self.error_type();

        (ty, span)
    }

    /// Parse optional lifetime arguments on a type use.
    pub(super) fn parse_type_lifetime_arguments(&mut self) -> ParseResult<Vec<Lifetime>> {
        if !self.peek_type_lifetime_arguments() {
            return Ok(Vec::new());
        }

        self.eat_token(TokenType::LessThan)?;
        let mut lifetimes = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            lifetimes.push(self.parse_lifetime_union()?);

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::GreaterThan)?;

        Ok(lifetimes)
    }

    /// Apply parsed lifetime arguments to one type use.
    pub(super) fn apply_type_lifetimes(
        &mut self,
        base: TypeId,
        lifetimes: Vec<Lifetime>,
    ) -> ParseResult<TypeId> {
        if lifetimes.is_empty() {
            return Ok(base);
        }

        self.intern_type(Type::WithLifetimes { base, lifetimes })
    }

    /// Return whether the next tokens start type lifetime arguments.
    fn peek_type_lifetime_arguments(&self) -> bool {
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }

        self.peek_nth_token(1)
            .is_some_and(|token| self.token_type(token) == TokenType::Lifetime)
    }

    /// Parse a type expression and append its span as one source segment.
    pub(super) fn parse_type_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<LocalNodeId<Type>> {
        let (ty, span) = self.parse_type_use_part()?;
        segment_spans.push(span);

        Ok(ty)
    }

    /// Check if a token can start a type in a value position.
    pub(super) fn peek_type(&self, kind: TokenType) -> bool {
        matches!(
            kind,
            TokenType::Void
                | TokenType::Boolean
                | TokenType::Identifier
                | TokenType::TypeName
                | TokenType::Ref
                | TokenType::TensorView
                | TokenType::Tensor
                | TokenType::Vector
                | TokenType::Newtype
                | TokenType::LessThan
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::OpenBrace
        )
    }

    /// Return whether a type position is structurally empty.
    fn is_missing_type_position(
        &self,
        delimiter: &Token,
        is_structural_type_allowed: bool,
    ) -> bool {
        let Some(token) = self.peek() else {
            return true;
        };
        let ty = self.token_type(token);
        let is_type_start = self.peek_type(ty);
        let is_structural_type = ty == TokenType::OpenBrace;

        // recover when the next declaration starts after a type hole
        if self.has_line_break_after(delimiter) && self.peek_identifier_label() {
            return true;
        }

        if is_type_start && (is_structural_type_allowed || !is_structural_type) {
            return false;
        }

        if self.has_line_break_after(delimiter) {
            return true;
        }

        matches!(
            ty,
            TokenType::CloseParenthesis
                | TokenType::OpenBrace
                | TokenType::CloseBrace
                | TokenType::CloseBracket
                | TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Equal
                | TokenType::End
        )
    }

    /// Return whether the next tokens look like a label or named declaration.
    fn peek_identifier_label(&self) -> bool {
        let Some(name) = self.peek_nth_token(0) else {
            return false;
        };

        let Some(delimiter) = self.peek_nth_token(1) else {
            return false;
        };

        self.token_type(name) == TokenType::Identifier
            && self.token_type(delimiter) == TokenType::Colon
    }

    /// Return whether a return type starts with a complete structural type.
    pub(super) fn is_return_structural_type_start(&self) -> bool {
        let tokens = self.tree.tokens();
        let Some((open_index, open_token)) = tokens
            .iter()
            .enumerate()
            .skip(self.pos)
            .find(|(_, token)| !token.is_trivia())
        else {
            return false;
        };

        if self.token_type(open_token) != TokenType::OpenBrace {
            return true;
        }

        let mut depth = 0usize;
        let mut close_index = open_index;
        while let Some(token) = tokens.get(close_index) {
            match self.token_type(token) {
                TokenType::OpenBrace => depth += 1,
                TokenType::CloseBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                TokenType::End => return false,
                _ => {}
            }

            close_index += 1;
        }

        if depth != 0 {
            return false;
        }

        tokens
            .iter()
            .skip(close_index + 1)
            .find(|token| !token.is_trivia())
            .is_some_and(|token| {
                matches!(
                    self.token_type(token),
                    TokenType::OpenBrace | TokenType::Semicolon
                )
            })
    }

    /// Parse a type expression.
    pub(super) fn parse_type(&mut self) -> ParseResult<LocalNodeId<Type>> {
        let (kind, token_start, token_text) = {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("type", self.pos()))?;
            (
                self.token_type(token),
                token.start(),
                self.tree.source_text(token.span).to_string(),
            )
        };

        // primitive or composite type
        let ty = match kind {
            TokenType::Void => {
                self.bump();
                Type::Void
            }
            TokenType::Boolean => {
                self.bump();
                Type::Boolean
            }
            TokenType::Identifier | TokenType::TypeName => {
                return self.parse_named_type(&token_text, token_start);
            }
            TokenType::Ref => self.parse_reference_type()?,
            TokenType::TensorView => self.parse_tensor_view_type()?,
            TokenType::Tensor => self.parse_tensor_type()?,
            TokenType::Vector => self.parse_vector_type()?,
            TokenType::Newtype => self.parse_newtype_type()?,
            TokenType::LessThan => {
                return self.parse_lifetime_signature_type();
            }
            TokenType::OpenParenthesis => self.parse_parenthesized_type()?,
            TokenType::OpenBracket => self.parse_array_type()?,
            TokenType::OpenBrace => {
                let (type_id, _, _) = self.parse_struct_type()?;
                return Ok(type_id);
            }
            _ => {
                return Err(ParseError::unexpected("type", kind, token_start));
            }
        };

        self.intern_type(ty)
    }

    /// Parse a named type form.
    fn parse_named_type(&mut self, name: &str, start: usize) -> ParseResult<LocalNodeId<Type>> {
        if let Some(primitive) = self.parse_primitive_type(name) {
            self.bump();
            return self.intern_type(primitive);
        }

        let ty = match name {
            "fn" => self.parse_function_pointer_type()?,
            "slice" => self.parse_slice_type()?,
            "atomic" => self.parse_atomic_type()?,
            "dynamic" => self.parse_dynamic_type()?,
            "uninit" => self.parse_uninit_type()?,
            "variant" => self.parse_variant_type()?,
            _ => {
                if let Some(declaration_id) = self.type_declaration_map.get(name).copied() {
                    self.bump();
                    return Ok(declaration_id);
                }

                return Err(ParseError::invalid("type", start));
            }
        };

        self.intern_type(ty)
    }

    /// Parse a function pointer type.
    fn parse_function_pointer_type(&mut self) -> ParseResult<Type> {
        self.bump();
        let signature = self.parse_signature(Vec::new())?;

        Ok(Type::FunctionPointer { signature })
    }

    /// Parse a slice type.
    fn parse_slice_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (element, _) = self.parse_type_use_part()?;
        let (kind, lifetime, space, access, nullability) = self.parse_slice_qualifiers()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Slice {
            kind,
            lifetime,
            element,
            space,
            access,
            nullability,
        })
    }

    /// Parse an atomic type.
    fn parse_atomic_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (value, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Atomic { value })
    }

    /// Parse a dynamic erased value type.
    fn parse_dynamic_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (constraint, _) = self.parse_type_use_part()?;
        let nullability = self.parse_nullability()?.unwrap_or(Nullability::None);
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Dynamic {
            constraint,
            nullability,
        })
    }

    /// Parse a linear uninitialized allocation token type.
    fn parse_uninit_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (value, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Uninit { value })
    }

    /// Parse a vector type.
    fn parse_vector_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (element, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;

        let token = self.eat_token(TokenType::Integer)?;
        let token_text = self.tree.source_text(token.span).to_string();
        let lanes = token_text.parse().map_err(|_| {
            ParseError::invalid(&format!("vector lane count '{token_text}'"), token.start())
        })?;

        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Vector {
            element,
            lanes,
            copy: Copy::No,
        })
    }

    /// Parse a newtype wrapper type.
    fn parse_newtype_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (inner, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Newtype {
            inner,
            copy: Copy::No,
        })
    }

    /// Parse a tuple or function value type.
    fn parse_parenthesized_type(&mut self) -> ParseResult<Type> {
        let parameters = self.parse_parenthesized_type_parameters()?;

        if self.eat_token_if(TokenType::FatArrow) {
            let signature = self.parse_signature_result(Vec::new(), parameters)?;
            let environment = self.tree.ensure_function_environment_type();

            return Ok(Type::Function {
                signature,
                environment,
            });
        }

        // reject callable-only parameter obligations on tuple elements
        if parameters
            .iter()
            .any(|parameter| !parameter.obligations.is_empty())
        {
            return Err(ParseError::invalid(
                "tuple type parameter obligation",
                self.pos(),
            ));
        }

        Ok(Type::Tuple {
            elements: parameters
                .into_iter()
                .map(|parameter| parameter.ty)
                .collect(),
            copy: Copy::No,
        })
    }

    /// Parse an explicitly lifetime-polymorphic function signature type.
    fn parse_lifetime_signature_type(&mut self) -> ParseResult<LocalNodeId<Type>> {
        self.parse_lifetime_scope(|parser, lifetimes| parser.parse_signature(lifetimes))
    }

    /// Parse type parameters enclosed in parentheses.
    pub(super) fn parse_parenthesized_type_parameters(
        &mut self,
    ) -> ParseResult<Vec<SignatureParameter>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut parameters = Vec::new();

        while !self.peek_is(TokenType::CloseParenthesis) {
            let (ty, _) = self.parse_type_use_part()?;
            let obligations = self.parse_borrow_obligations()?;
            parameters.push(SignatureParameter { ty, obligations });

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(parameters)
    }

    /// Parse a function signature type.
    pub(super) fn parse_signature(
        &mut self,
        lifetimes: Vec<LifetimeParameter>,
    ) -> ParseResult<LocalNodeId<Type>> {
        let parameters = self.parse_parenthesized_type_parameters()?;
        self.eat_token(TokenType::FatArrow)?;

        self.parse_signature_result(lifetimes, parameters)
    }

    /// Parse a function signature result after its parameter types.
    fn parse_signature_result(
        &mut self,
        mut lifetimes: Vec<LifetimeParameter>,
        parameters: Vec<SignatureParameter>,
    ) -> ParseResult<LocalNodeId<Type>> {
        let (result, _) = self.parse_type_use_part()?;
        self.parse_lifetime_where(&mut lifetimes)?;
        self.intern_type(Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        })
    }

    /// Parse a fixed array type.
    fn parse_array_type(&mut self) -> ParseResult<Type> {
        self.bump();
        let (element, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Semicolon)?;

        let length = self.parse_int_literal()?;
        let length =
            u64::try_from(length).map_err(|_| ParseError::invalid("array length", self.pos()))?;

        self.eat_token(TokenType::CloseBracket)?;

        Ok(Type::FixedArray {
            element,
            length,
            copy: Copy::No,
        })
    }

    /// Parse one struct type and retain field declaration spans.
    pub(super) fn parse_struct_type(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Type>, Vec<FieldSpan>, TypeDeclarationSpans)> {
        let open_brace_token = self.eat_token(TokenType::OpenBrace)?;
        let open_brace_start = open_brace_token.start();
        let open_brace_length = self.tree.source_text(open_brace_token.span).len();
        let open_brace_span = self.span_at(open_brace_start, open_brace_length);

        let mut fields = Vec::new();
        let mut field_spans = Vec::new();

        while !self.peek_is(TokenType::CloseBrace) {
            let field_start = self.pos();

            // field attributes
            let (attributes, attribute_spans) = self.parse_attributes()?;

            // field name
            let mut name = None;
            let mut name_span = None;
            let mut type_anchor = None;
            if self.peek_is(TokenType::At)
                && self
                    .peek_nth_token(1)
                    .is_some_and(|token| self.token_type(token) == TokenType::Identifier)
                && self
                    .peek_nth_token(2)
                    .is_some_and(|token| self.token_type(token) == TokenType::Colon)
            {
                self.eat_token(TokenType::At)?;
                let name_token = self.eat_token(TokenType::Identifier)?;
                let name_start = name_token.start();
                let name_text = self.tree.source_text(name_token.span).to_string();
                let name_length = name_text.len() + 1;
                let display_name = format!("@{name_text}");
                let span = self.span_at(name_start, name_length);
                type_anchor = Some(self.eat_token(TokenType::Colon)?);
                name = Some(self.strings.intern(&display_name));
                name_span = Some(span);
            } else if self.peek_is(TokenType::Identifier)
                && let Some(next_token) = self.peek_nth_token(1)
                && self.token_type(next_token) == TokenType::Colon
            {
                let name_token = self.eat_token(TokenType::Identifier)?;
                let name_start = name_token.start();
                let name_text = self.tree.source_text(name_token.span).to_string();
                let name_length = name_text.len();
                let span = self.span_at(name_start, name_length);
                type_anchor = Some(self.eat_token(TokenType::Colon)?);
                name = Some(self.strings.intern(&name_text));
                name_span = Some(span);
            }

            // field type
            let is_line_hole = type_anchor
                .as_ref()
                .is_some_and(|token| self.has_line_break_after(token));
            let (ty, type_span) = if let Some(type_anchor) = type_anchor {
                self.parse_type_use_after(type_anchor, "field type")
            } else {
                self.parse_type_use_part()?
            };

            // field node
            let field = Field { name, ty };
            fields.push(self.tree.intern_field(field, attributes));

            // field delimiter
            if self.eat_token_if(TokenType::Semicolon) || self.eat_token_if(TokenType::Comma) {
                let field_span = self.span_from_parse_start(field_start);
                field_spans.push(FieldSpan::new(
                    field_span,
                    attribute_spans,
                    name_span,
                    type_span,
                ));
                continue;
            }

            if is_line_hole {
                let field_span = self.span_from_parse_start(field_start);
                field_spans.push(FieldSpan::new(
                    field_span,
                    attribute_spans,
                    name_span,
                    type_span,
                ));
                continue;
            }

            if self.peek_is(TokenType::CloseBrace) {
                let field_span = self.span_from_parse_start(field_start);
                field_spans.push(FieldSpan::new(
                    field_span,
                    attribute_spans,
                    name_span,
                    type_span,
                ));
                break;
            }

            return Err(ParseError::invalid("';' or '}'", self.pos()));
        }

        let close_brace_token = self.eat_token(TokenType::CloseBrace)?;
        let close_brace_start = close_brace_token.start();
        let close_brace_length = self.tree.source_text(close_brace_token.span).len();
        let close_brace_span = self.span_at(close_brace_start, close_brace_length);

        let struct_type = Type::Struct {
            fields,
            copy: Copy::No,
        };
        let type_id = self.intern_type(struct_type)?;

        Ok((
            type_id,
            field_spans,
            TypeDeclarationSpans::new(None, Some(open_brace_span), Some(close_brace_span)),
        ))
    }

    /// Parse a reference type.
    fn parse_reference_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;

        let (pointee, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers(Nullability::None)?;

        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Reference {
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            space: qualifiers.space,
            access: qualifiers.access,
            pointee,
            nullability: qualifiers.nullability,
        })
    }

    /// Parse a tensor view type.
    fn parse_tensor_view_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;

        let (element, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers(Nullability::None)?;

        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let mut format = TensorViewFormat::dense_row_major();
        let mut sharding = TensorSharding::unsharded();
        self.parse_optional_tensor_groups(None, Some(&mut format), &mut sharding)?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::TensorView {
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            space: qualifiers.space,
            access: qualifiers.access,
            element,
            shape,
            format,
            sharding,
            nullability: qualifiers.nullability,
        })
    }

    /// Parse a physical variant type.
    fn parse_variant_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let discriminant = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;
        let (storage, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;
        self.eat_token(TokenType::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.peek_is(TokenType::CloseBrace) {
            let discriminant = self.parse_constant_for_type(discriminant)?;
            self.eat_token(TokenType::Equal)?;
            let (ty, _) = self.parse_type_use_part()?;
            cases.push(VariantCase { discriminant, ty });

            if self.eat_token_if(TokenType::Semicolon) || self.eat_token_if(TokenType::Comma) {
                continue;
            }

            if self.peek_is(TokenType::CloseBrace) {
                break;
            }

            return Err(ParseError::invalid("';' or '}'", self.pos()));
        }

        self.eat_token(TokenType::CloseBrace)?;

        Ok(Type::Variant {
            discriminant,
            storage,
            cases,
            copy: Copy::No,
        })
    }

    /// Parse a tensor type.
    fn parse_tensor_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (element, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let mut format = TensorFormat::dense_row_major();
        let mut sharding = TensorSharding::unsharded();
        self.parse_optional_tensor_groups(Some(&mut format), None, &mut sharding)?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Tensor {
            element,
            shape,
            format,
            sharding,
            copy: Copy::No,
        })
    }

    /// Parse reference-like qualifiers after the pointee type.
    fn parse_reference_qualifiers(
        &mut self,
        nullability: Nullability,
    ) -> ParseResult<ResolvedReferenceQualifiers> {
        let mut qualifiers = ReferenceQualifiers::new(nullability);
        self.parse_reference_qualifier(&mut qualifiers)?;

        while self.peek_is(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| self.token_type(token) == TokenType::OpenParenthesis)
            {
                break;
            }

            self.bump();
            self.parse_reference_qualifier(&mut qualifiers)?;
        }

        qualifiers.finish("reference kind", self.pos())
    }

    /// Parse optional trailing qualifiers for one slice type.
    fn parse_slice_qualifiers(
        &mut self,
    ) -> ParseResult<(ReferenceKind, Lifetime, Space, Access, Nullability)> {
        let mut qualifiers = ReferenceQualifiers::new(Nullability::None);

        while self.peek_is(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| self.token_type(token) == TokenType::OpenParenthesis)
            {
                break;
            }

            self.bump();
            self.parse_reference_qualifier(&mut qualifiers)?;
        }

        let qualifiers = qualifiers.finish("slice kind", self.pos())?;

        Ok((
            qualifiers.kind,
            qualifiers.lifetime,
            qualifiers.space,
            qualifiers.access,
            qualifiers.nullability,
        ))
    }

    /// Parse one reference-like qualifier.
    fn parse_reference_qualifier(
        &mut self,
        qualifiers: &mut ReferenceQualifiers,
    ) -> ParseResult<()> {
        if let Some(kind) = self.parse_reference_kind()? {
            qualifiers.kind = Some(kind);
            return Ok(());
        }

        if self.eat_token_if(TokenType::Readonly) {
            qualifiers.access = Some(Access::Readonly);
            return Ok(());
        }

        if self.eat_name_if("mutable") {
            qualifiers.access = Some(Access::Mutable);
            return Ok(());
        }

        if self.eat_name_if("exclusive") {
            qualifiers.access = Some(Access::Exclusive);
            return Ok(());
        }

        if let Some(nullability) = self.parse_nullability()? {
            qualifiers.nullability = nullability;
            return Ok(());
        }

        if self
            .peek()
            .is_some_and(|token| self.token_type(token) == TokenType::Lifetime)
        {
            qualifiers.lifetime = self.parse_lifetime_union()?;
            return Ok(());
        }

        if self.eat_token_if(TokenType::Space) {
            qualifiers.space = self.parse_space_group()?;
            return Ok(());
        }

        Err(ParseError::invalid("reference qualifier", self.pos()))
    }

    /// Parse one reference ownership kind.
    fn parse_reference_kind(&mut self) -> ParseResult<Option<ReferenceKind>> {
        let Some(token) = self.peek() else {
            return Ok(None);
        };
        if !matches!(
            self.token_type(token),
            TokenType::Ownership | TokenType::Identifier
        ) {
            return Ok(None);
        }

        let kind = match self.tree.source_text(token.span) {
            "managed" => ReferenceKind::Managed,
            "unique" => ReferenceKind::Unique,
            "borrowed" => ReferenceKind::Borrowed,
            "raw" => ReferenceKind::Raw,
            _ => return Ok(None),
        };
        self.bump();

        Ok(Some(kind))
    }

    /// Parse one nullish qualifier.
    fn parse_nullability(&mut self) -> ParseResult<Option<Nullability>> {
        let Some(token) = self.peek() else {
            return Ok(None);
        };
        if self.token_type(token) != TokenType::Identifier {
            return Ok(None);
        }

        let nullability = match self.tree.source_text(token.span) {
            "nullable" => Nullability::Null,
            "undefined" => Nullability::Undefined,
            "nullish" => Nullability::NullOrUndefined,
            _ => return Ok(None),
        };
        self.bump();

        Ok(Some(nullability))
    }

    /// Parse one address-space qualifier group.
    fn parse_space_group(&mut self) -> ParseResult<Space> {
        self.eat_token(TokenType::OpenParenthesis)?;

        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("space", self.pos()))?;
        let space = match self.token_type(token) {
            TokenType::Identifier | TokenType::Local => {
                let text = self.tree.source_text(token.span);
                Space::from_name(text).ok_or_else(|| {
                    ParseError::invalid_with_length(
                        "space",
                        token.start(),
                        token.span.len() as usize,
                    )
                })?
            }
            _ => {
                return Err(ParseError::unexpected(
                    "space",
                    self.token_type(token),
                    token.start(),
                ));
            }
        };
        self.bump();
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(space)
    }

    /// Parse one tick lifetime union.
    pub(super) fn parse_lifetime_union(&mut self) -> ParseResult<Lifetime> {
        let mut terms = vec![self.parse_lifetime_term()?];
        while self.eat_token_if(TokenType::Pipe) {
            terms.push(self.parse_lifetime_term()?);
        }

        Ok(Lifetime::new(terms))
    }

    /// Parse one tick lifetime term.
    fn parse_lifetime_term(&mut self) -> ParseResult<LifetimeTerm> {
        let token = self.eat_token(TokenType::Lifetime)?;
        let name = self.tree.source_text(token.span);
        if name == "'static" {
            return Ok(LifetimeTerm::Static);
        }

        let Some(slot) = self.lifetime_slot(name) else {
            return Err(ParseError::invalid_with_length(
                "lifetime name",
                token.start(),
                token.span.len() as usize,
            ));
        };

        Ok(LifetimeTerm::Slot(slot))
    }

    /// Parse function signature borrow obligations.
    pub(super) fn parse_borrow_obligations(&mut self) -> ParseResult<Vec<BorrowObligation>> {
        let mut obligations = Vec::new();

        // parse trailing suspension source requirements
        while self.peek_borrow_obligation() {
            self.eat_token(TokenType::At)?;
            self.eat_token(TokenType::Identifier)?;

            self.eat_token(TokenType::OpenParenthesis)?;
            let lifetime = self.parse_lifetime_union()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            obligations.push(BorrowObligation::SuspensionStable { lifetime });
        }

        Ok(obligations)
    }

    /// Return whether the next tokens start a borrow obligation.
    fn peek_borrow_obligation(&self) -> bool {
        if !self.peek_is(TokenType::At) {
            return false;
        }

        self.peek_nth_token(1).is_some_and(|token| {
            self.token_type(token) == TokenType::Identifier
                && self.tree.source_text(token.span) == "suspensionSafe"
        })
    }

    /// Parse a tensor shape list.
    fn parse_tensor_shape(&mut self) -> ParseResult<Vec<TensorDimension>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut shape = Vec::new();
        while !self.peek_is(TokenType::CloseParenthesis) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("tensor shape", self.pos()))?;
            match self.token_type(token) {
                TokenType::Integer => {
                    let dim = self.parse_int_literal()?;
                    let dim = u64::try_from(dim)
                        .map_err(|_| ParseError::invalid("tensor shape dimension", self.pos()))?;
                    shape.push(TensorDimension::Static(dim));
                }
                TokenType::Identifier => {
                    let ident = self.eat_token(TokenType::Identifier)?;
                    let text = self.tree.source_text(ident.span);
                    if text == "dynamic" {
                        shape.push(TensorDimension::Dynamic);
                    } else {
                        shape.push(TensorDimension::Symbol(text.to_string()));
                    }
                }
                _ => {
                    return Err(ParseError::unexpected(
                        "tensor shape dimension",
                        self.token_type(token),
                        token.start(),
                    ));
                }
            }
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(shape)
    }

    /// Parse optional tensor format and sharding groups.
    fn parse_optional_tensor_groups(
        &mut self,
        mut format: Option<&mut TensorFormat>,
        mut view_format: Option<&mut TensorViewFormat>,
        sharding: &mut TensorSharding,
    ) -> ParseResult<()> {
        while self.eat_token_if(TokenType::Comma) {
            let token = self.eat_token(TokenType::Identifier)?;
            match self.tree.source_text(token.span) {
                "format" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    if let Some(format) = format.as_deref_mut() {
                        *format = self.parse_tensor_format()?;
                    } else if let Some(view_format) = view_format.as_deref_mut() {
                        *view_format = self.parse_tensor_view_format()?;
                    } else {
                        return Err(ParseError::invalid("tensor format group", token.start()));
                    }
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "sharding" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    *sharding = self.parse_tensor_sharding()?;
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                _ => return Err(ParseError::invalid("tensor group", token.start())),
            }
        }

        Ok(())
    }

    /// Parse a tensor format specifier.
    fn parse_tensor_format(&mut self) -> ParseResult<TensorFormat> {
        let token = self.eat_token(TokenType::Identifier)?;
        match self.tree.source_text(token.span) {
            "dense" => {
                self.eat_token(TokenType::OpenParenthesis)?;
                let order = self.parse_tensor_dimension_order()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Ok(TensorFormat::Dense { order })
            }
            _ => Err(ParseError::invalid("tensor format", token.start())),
        }
    }

    /// Parse a tensor view format specifier.
    fn parse_tensor_view_format(&mut self) -> ParseResult<TensorViewFormat> {
        let token = self.eat_token(TokenType::Identifier)?;
        match self.tree.source_text(token.span) {
            "dense" => {
                self.eat_token(TokenType::OpenParenthesis)?;
                let order = self.parse_tensor_dimension_order()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Ok(TensorViewFormat::Dense { order })
            }
            "strided" => Ok(TensorViewFormat::Strided),
            _ => Err(ParseError::invalid("tensor view format", token.start())),
        }
    }

    /// Parse a tensor sharding specifier.
    fn parse_tensor_sharding(&mut self) -> ParseResult<TensorSharding> {
        let token = self.eat_token(TokenType::Identifier)?;
        match self.tree.source_text(token.span) {
            "unsharded" => Ok(TensorSharding::Unsharded),
            _ => {
                let mut axes = vec![self.parse_tensor_sharding_axis_after(token)?];
                while self.eat_token_if(TokenType::Comma) {
                    axes.push(self.parse_tensor_sharding_axis()?);
                }

                Ok(TensorSharding::Sharding { axes })
            }
        }
    }

    /// Parse one tensor sharding axis.
    fn parse_tensor_sharding_axis(&mut self) -> ParseResult<TensorShardingAxis> {
        let token = self.eat_token(TokenType::Identifier)?;

        self.parse_tensor_sharding_axis_after(token)
    }

    /// Parse one tensor sharding axis after its leading token has been consumed.
    fn parse_tensor_sharding_axis_after(
        &mut self,
        token: Token,
    ) -> ParseResult<TensorShardingAxis> {
        match self.tree.source_text(token.span) {
            "shard" => {
                self.eat_token(TokenType::OpenParenthesis)?;
                let axis = self.parse_int_literal()?;
                let axis = i32::try_from(axis)
                    .map_err(|_| ParseError::invalid("tensor shard axis", self.pos()))?;
                self.eat_token(TokenType::CloseParenthesis)?;

                Ok(TensorShardingAxis::Shard { axis })
            }
            "replicate" => Ok(TensorShardingAxis::Replicate),
            "partial" => {
                self.eat_token(TokenType::OpenParenthesis)?;
                let reduction = self.parse_tensor_reduction()?;
                self.eat_token(TokenType::CloseParenthesis)?;

                Ok(TensorShardingAxis::Partial { reduction })
            }
            _ => Err(ParseError::invalid("tensor sharding axis", token.start())),
        }
    }

    /// Parse one partial tensor reduction.
    fn parse_tensor_reduction(&mut self) -> ParseResult<TensorReduction> {
        let token = self.eat_token(TokenType::Identifier)?;
        let reduction = match self.tree.source_text(token.span) {
            "add" => TensorReduction::Add,
            "multiply" => TensorReduction::Multiply,
            "minimum" => TensorReduction::Minimum,
            "maximum" => TensorReduction::Maximum,
            "and" => TensorReduction::And,
            "or" => TensorReduction::Or,
            _ => return Err(ParseError::invalid("tensor reduction", token.start())),
        };

        Ok(reduction)
    }

    /// Parse one dense tensor dimension order.
    fn parse_tensor_dimension_order(&mut self) -> ParseResult<TensorDimensionOrder> {
        let token = self.eat_token(TokenType::Identifier)?;
        let order = match self.tree.source_text(token.span) {
            "rowMajor" => TensorDimensionOrder::RowMajor,
            "columnMajor" => TensorDimensionOrder::ColumnMajor,
            _ => return Err(ParseError::invalid("tensor dimension order", token.start())),
        };

        Ok(order)
    }

    /// Return a canonical type id for the provided type shape.
    pub(super) fn intern_type(&mut self, ty: Type) -> ParseResult<LocalNodeId<Type>> {
        Ok(self.tree.intern_type(ty))
    }

    /// Return the canonical parse-recovery type.
    pub(super) fn error_type(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Error)
    }
}
