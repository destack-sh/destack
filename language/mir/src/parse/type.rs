use crate::source::{Token, TokenType};
use destack_source::Span;

use crate::{
    Access, Attribute, BorrowObligation, Copy, Field, FieldSpan, Lifetime, LifetimeOrigin,
    LocalNodeId, Nullability, ReferenceKind, Space, TensorDimension, TensorDimensionOrder,
    TensorLayout, TensorViewLayout, Type, TypeDeclarationSpans, TypeReference, VariantCase,
};

use super::error::{ParseError, ParseResult};
use super::key::{FieldKey, TypeKey};
use super::parser::Parser;

/// Parsed reference-like type qualifiers.
#[derive(Debug, Clone)]
struct ReferenceQualifiers {
    /// The reference ownership kind.
    kind: Option<ReferenceKind>,
    /// The explicit borrowed lifetime.
    lifetime: Lifetime,
    /// The referenced space.
    space: Space,
    /// The exposed access mode.
    access: Access,
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
            access: Access::Mutable,
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

        if kind != ReferenceKind::Borrowed && !self.lifetime.is_empty() {
            return Err(ParseError::invalid("borrowed lifetime", pos));
        }

        Ok(ResolvedReferenceQualifiers {
            kind,
            lifetime: self.lifetime,
            space: self.space,
            access: self.access,
            nullability: self.nullability,
        })
    }
}

/// Resolved reference-like type qualifiers.
#[derive(Debug, Clone)]
struct ResolvedReferenceQualifiers {
    /// The reference ownership kind.
    kind: ReferenceKind,
    /// The explicit borrowed lifetime.
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

    /// Parse a type reference after one required delimiter.
    pub(super) fn parse_type_reference_after(
        &mut self,
        delimiter: Token,
        expected: &'static str,
    ) -> (TypeReference, Span) {
        self.parse_type_reference_after_with_structural_type(delimiter, expected, true)
    }

    /// Parse a return type reference after its required delimiter.
    pub(super) fn parse_return_type_reference_after(
        &mut self,
        delimiter: Token,
    ) -> (TypeReference, Span) {
        let is_structural_type_allowed = self.is_return_structural_type_start();

        self.parse_type_reference_after_with_structural_type(
            delimiter,
            "return type",
            is_structural_type_allowed,
        )
    }

    /// Parse a type reference after one required delimiter.
    fn parse_type_reference_after_with_structural_type(
        &mut self,
        delimiter: Token,
        expected: &'static str,
        is_structural_type_allowed: bool,
    ) -> (TypeReference, Span) {
        let hole_position = delimiter.span.end as usize;

        if self.is_missing_type_position(&delimiter, is_structural_type_allowed) {
            let error = ParseError::new(format!("expected {expected}"), hole_position);
            self.diagnostics
                .insert(error.to_diagnostic(self.content_id, self.file_id));

            return (TypeReference::Missing, self.span_at(hole_position, 0));
        }

        let type_start = self.pos();
        match self.parse_type() {
            Ok(ty) => {
                let span = self.span_from_parse_start(type_start);

                (TypeReference::Type(ty), span)
            }
            Err(error) => {
                self.diagnostics
                    .insert(error.to_diagnostic(self.content_id, self.file_id));

                if self.peek().is_some() {
                    self.bump();
                }

                let error_end = self.pos();
                let span = self.span_at(error.position, error_end.saturating_sub(error.position));

                (TypeReference::Error, span)
            }
        }
    }

    /// Parse a type expression and append its span as one source segment.
    pub(super) fn parse_type_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<LocalNodeId<Type>> {
        let (ty, span) = self.parse_type_part()?;
        segment_spans.push(span);

        Ok(ty)
    }

    /// Check if a token can start a type in a value position.
    pub(super) fn peek_type(&self, kind: TokenType) -> bool {
        matches!(
            kind,
            TokenType::Void
                | TokenType::Boolean
                | TokenType::TypeName
                | TokenType::Ref
                | TokenType::TensorView
                | TokenType::Tensor
                | TokenType::Vector
                | TokenType::Newtype
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

    /// Return whether a return type starts with a complete structural type.
    fn is_return_structural_type_start(&self) -> bool {
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
                    depth = depth.saturating_sub(1);
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
                token.start,
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
            "slice" => self.parse_slice_type()?,
            "atomic" => self.parse_atomic_type()?,
            "dynamic" => self.parse_dynamic_type()?,
            "uninit" => self.parse_uninit_type()?,
            "variant" => self.parse_variant_type()?,
            _ => {
                if let Some(alias_id) = self.type_alias_map.get(name).copied() {
                    self.bump();
                    return Ok(alias_id);
                }

                return Err(ParseError::invalid("type", start));
            }
        };

        self.intern_type(ty)
    }

    /// Parse a slice type.
    fn parse_slice_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let element = self.parse_type()?;
        let (kind, lifetime, space, access, nullability) = self.parse_slice_qualifiers()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Slice {
            kind,
            lifetime,
            element: element.into(),
            space,
            access,
            nullability,
        })
    }

    /// Parse an atomic type.
    fn parse_atomic_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let value = self.parse_type()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Atomic {
            value: value.into(),
        })
    }

    /// Parse a dynamic erased value type.
    fn parse_dynamic_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let constraint = self.parse_type()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Dynamic {
            constraint: constraint.into(),
        })
    }

    /// Parse a linear uninitialized allocation token type.
    fn parse_uninit_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let value = self.parse_type()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Uninit {
            value: value.into(),
        })
    }

    /// Parse a vector type.
    fn parse_vector_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let element = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;

        let token = self.eat_token(TokenType::Integer)?;
        let token_text = self.tree.source_text(token.span).to_string();
        let lanes = token_text.parse().map_err(|_| {
            ParseError::invalid(&format!("vector lane count '{token_text}'"), token.start)
        })?;

        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Vector {
            element: element.into(),
            lanes,
            copy: Copy::default(),
        })
    }

    /// Parse a newtype wrapper type.
    fn parse_newtype_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let inner = self.parse_type()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Newtype {
            inner: inner.into(),
            copy: Copy::default(),
        })
    }

    /// Parse a tuple, function pointer, or callable type.
    fn parse_parenthesized_type(&mut self) -> ParseResult<Type> {
        self.bump();
        let mut parameters = Vec::new();

        while !self.peek_token(TokenType::CloseParenthesis) {
            parameters.push(self.parse_type()?.into());
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;

        if self.eat_token_maybe(TokenType::Arrow) {
            let signature = self.parse_function_signature(parameters)?;

            return Ok(Type::FunctionPointer {
                signature: signature.into(),
            });
        }

        if self.eat_token_maybe(TokenType::FatArrow) {
            let signature = self.parse_function_signature(parameters)?;
            let environment = self.tree.ensure_closure_environment_type();

            return Ok(Type::Closure {
                signature: signature.into(),
                environment: environment.into(),
            });
        }

        Ok(Type::Tuple {
            elements: parameters,
            copy: Copy::default(),
        })
    }

    /// Parse a function signature result after parameter types.
    fn parse_function_signature(
        &mut self,
        parameters: Vec<TypeReference>,
    ) -> ParseResult<LocalNodeId<Type>> {
        let result = self.parse_type()?;
        let borrow_obligations = self.parse_borrow_obligations()?;
        self.intern_type(Type::FunctionSignature {
            parameters,
            result: result.into(),
            borrow_obligations,
        })
    }

    /// Parse a fixed-size array type.
    fn parse_array_type(&mut self) -> ParseResult<Type> {
        self.bump();
        let element = self.parse_type()?;
        self.eat_token(TokenType::Semicolon)?;

        let length = self.parse_int_literal()?;
        let length =
            u64::try_from(length).map_err(|_| ParseError::invalid("array length", self.pos()))?;

        self.eat_token(TokenType::CloseBracket)?;

        Ok(Type::Array {
            element: element.into(),
            length,
            copy: Copy::default(),
        })
    }

    /// Parse one struct type and retain field declaration spans.
    pub(super) fn parse_struct_type(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Type>, Vec<FieldSpan>, TypeDeclarationSpans)> {
        let open_brace_token = self.eat_token(TokenType::OpenBrace)?;
        let open_brace_start = open_brace_token.start;
        let open_brace_length = self.tree.source_text(open_brace_token.span).len();
        let open_brace_span = self.span_at(open_brace_start, open_brace_length);

        let mut fields = Vec::new();
        let mut field_spans = Vec::new();

        while !self.peek_token(TokenType::CloseBrace) {
            let field_start = self.pos();

            // field attributes
            let (attributes, attribute_spans) = self.parse_attributes()?;

            // field name
            let mut name = None;
            let mut name_span = None;
            let mut type_anchor = None;
            if self.peek_token(TokenType::At)
                && self
                    .peek_nth_token(1)
                    .is_some_and(|token| self.token_type(token) == TokenType::Identifier)
                && self
                    .peek_nth_token(2)
                    .is_some_and(|token| self.token_type(token) == TokenType::Colon)
            {
                self.eat_token(TokenType::At)?;
                let name_token = self.eat_token(TokenType::Identifier)?;
                let name_start = name_token.start;
                let name_text = self.tree.source_text(name_token.span).to_string();
                let name_length = name_text.len() + 1;
                let display_name = format!("@{name_text}");
                let span = self.span_at(name_start, name_length);
                type_anchor = Some(self.eat_token(TokenType::Colon)?);
                name = Some(self.strings.intern(&display_name));
                name_span = Some(span);
            } else if self.peek_token(TokenType::Identifier)
                && let Some(next_token) = self.peek_nth_token(1)
                && self.token_type(next_token) == TokenType::Colon
            {
                let name_token = self.eat_token(TokenType::Identifier)?;
                let name_start = name_token.start;
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
                self.parse_type_reference_after(type_anchor, "field type")
            } else {
                let (ty, type_span) = self.parse_type_part()?;
                (TypeReference::Type(ty), type_span)
            };

            // field node
            let field = Field { name, ty };
            fields.push(self.intern_field(field, attributes));

            // field delimiter
            if self.eat_token_maybe(TokenType::Semicolon) || self.eat_token_maybe(TokenType::Comma)
            {
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

            if self.peek_token(TokenType::CloseBrace) {
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
        let close_brace_start = close_brace_token.start;
        let close_brace_length = self.tree.source_text(close_brace_token.span).len();
        let close_brace_span = self.span_at(close_brace_start, close_brace_length);

        let struct_type = Type::Struct {
            fields,
            copy: Copy::default(),
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

        let pointee = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers(Nullability::None)?;

        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Reference {
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            space: qualifiers.space,
            access: qualifiers.access,
            pointee: pointee.into(),
            nullability: qualifiers.nullability,
        })
    }

    /// Parse a tensor view type.
    fn parse_tensor_view_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;

        let element = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers(Nullability::None)?;

        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let layout = self.parse_optional_tensor_view_layout()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::TensorView {
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            space: qualifiers.space,
            access: qualifiers.access,
            element: element.into(),
            shape,
            layout,
            nullability: qualifiers.nullability,
        })
    }

    /// Parse a physical variant type.
    fn parse_variant_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let tag_type = self.parse_type()?;
        let tag = tag_type.into();
        self.eat_token(TokenType::Comma)?;
        let storage = self.parse_type()?.into();
        self.eat_token(TokenType::GreaterThan)?;
        self.eat_token(TokenType::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.peek_token(TokenType::CloseBrace) {
            let tag = self.parse_constant_for_type(tag_type)?;
            self.eat_token(TokenType::Equal)?;
            let ty = self.parse_type()?.into();
            cases.push(VariantCase { tag, ty });

            if self.eat_token_maybe(TokenType::Semicolon) || self.eat_token_maybe(TokenType::Comma)
            {
                continue;
            }

            if self.peek_token(TokenType::CloseBrace) {
                break;
            }

            return Err(ParseError::invalid("';' or '}'", self.pos()));
        }

        self.eat_token(TokenType::CloseBrace)?;

        Ok(Type::Variant {
            tag,
            storage,
            cases,
            copy: Copy::default(),
        })
    }

    /// Parse a tensor type.
    fn parse_tensor_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let element = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let layout = self.parse_optional_tensor_layout()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Tensor {
            element: element.into(),
            shape,
            layout,
            copy: Copy::default(),
        })
    }

    /// Parse reference-like qualifiers after the pointee type.
    fn parse_reference_qualifiers(
        &mut self,
        nullability: Nullability,
    ) -> ParseResult<ResolvedReferenceQualifiers> {
        let mut qualifiers = ReferenceQualifiers::new(nullability);
        self.parse_reference_qualifier(&mut qualifiers)?;

        while self.peek_token(TokenType::Comma) {
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

        while self.peek_token(TokenType::Comma) {
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

        if self.eat_token_maybe(TokenType::Readonly) {
            qualifiers.access = Access::Readonly;
            return Ok(());
        }

        if self.eat_identifier_text("exclusive") {
            qualifiers.access = Access::Exclusive;
            return Ok(());
        }

        if let Some(nullability) = self.parse_nullability()? {
            qualifiers.nullability = nullability;
            return Ok(());
        }

        if self.eat_identifier_text("lifetime") {
            qualifiers.lifetime = self.parse_lifetime_group()?;
            return Ok(());
        }

        if self.eat_token_maybe(TokenType::Space) {
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
                    ParseError::invalid_at_span(
                        "space",
                        token.start,
                        token.span.end.saturating_sub(token.span.start) as usize,
                    )
                })?
            }
            _ => {
                return Err(ParseError::unexpected(
                    "space",
                    self.token_type(token),
                    token.start,
                ));
            }
        };
        self.bump();
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(space)
    }

    /// Parse a lifetime qualifier group.
    fn parse_lifetime_group(&mut self) -> ParseResult<Lifetime> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut origins = Vec::new();

        while !self.peek_token(TokenType::CloseParenthesis) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("lifetime", self.pos()))?;
            match self.token_type(token) {
                TokenType::Identifier if self.tree.source_text(token.span) == "static" => {
                    origins.push(LifetimeOrigin::Static);
                    self.bump();
                }
                TokenType::Integer => {
                    let index = self.parse_int_literal()?;
                    let index = u32::try_from(index)
                        .map_err(|_| ParseError::invalid("lifetime parameter", self.pos()))?;
                    origins.push(LifetimeOrigin::Parameter(index));
                }
                _ => {
                    return Err(ParseError::unexpected(
                        "lifetime origin",
                        self.token_type(token),
                        token.start,
                    ));
                }
            }

            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;
        if origins.is_empty() {
            return Err(ParseError::invalid("lifetime origin", self.pos()));
        }

        Ok(Lifetime::new(origins))
    }

    /// Parse function signature borrow obligations.
    pub(super) fn parse_borrow_obligations(&mut self) -> ParseResult<Vec<BorrowObligation>> {
        let mut obligations = Vec::new();

        // parse trailing suspension source requirements
        while self.peek_token(TokenType::At) {
            self.eat_token(TokenType::At)?;
            let name_token = self.eat_token(TokenType::Identifier)?;
            let name_text = self.tree.source_text(name_token.span);
            if name_text != "suspensionSafe" {
                return Err(ParseError::invalid(
                    &format!("borrow obligation '@{name_text}'"),
                    name_token.start,
                ));
            }

            let lifetime = self.parse_lifetime_group()?;
            obligations.push(BorrowObligation::SuspensionStable { lifetime });
        }

        Ok(obligations)
    }

    /// Parse an optional trailing tensor layout assignment.
    fn parse_optional_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        if self.eat_token_maybe(TokenType::Comma) {
            self.parse_tensor_layout_group()
        } else {
            Ok(TensorLayout::dense_row_major())
        }
    }

    /// Parse an optional trailing tensor view layout assignment.
    fn parse_optional_tensor_view_layout(&mut self) -> ParseResult<TensorViewLayout> {
        if self.eat_token_maybe(TokenType::Comma) {
            self.parse_tensor_view_layout_group()
        } else {
            Ok(TensorViewLayout::dense_row_major())
        }
    }

    /// Parse a tensor shape list.
    fn parse_tensor_shape(&mut self) -> ParseResult<Vec<TensorDimension>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut shape = Vec::new();
        while !self.peek_token(TokenType::CloseParenthesis) {
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
                        token.start,
                    ));
                }
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(shape)
    }

    /// Parse a grouped tensor layout clause.
    fn parse_tensor_layout_group(&mut self) -> ParseResult<TensorLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        if self.tree.source_text(token.span) != "layout" {
            return Err(ParseError::invalid("layout group", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;
        let layout = self.parse_tensor_layout()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(layout)
    }

    /// Parse a grouped tensor view layout clause.
    fn parse_tensor_view_layout_group(&mut self) -> ParseResult<TensorViewLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        if self.tree.source_text(token.span) != "layout" {
            return Err(ParseError::invalid("layout group", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;
        let layout = self.parse_tensor_view_layout()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(layout)
    }

    /// Parse a tensor layout specifier.
    fn parse_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        match self.tree.source_text(token.span) {
            "dense" => {
                self.eat_token(TokenType::OpenParenthesis)?;
                let order = self.parse_tensor_dimension_order()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Ok(TensorLayout::Dense { order })
            }
            _ => Err(ParseError::invalid("tensor layout", token.start)),
        }
    }

    /// Parse a tensor view layout specifier.
    fn parse_tensor_view_layout(&mut self) -> ParseResult<TensorViewLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        match self.tree.source_text(token.span) {
            "dense" => {
                self.eat_token(TokenType::OpenParenthesis)?;
                let order = self.parse_tensor_dimension_order()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Ok(TensorViewLayout::Dense { order })
            }
            "strided" => Ok(TensorViewLayout::Strided),
            _ => Err(ParseError::invalid("tensor view layout", token.start)),
        }
    }

    /// Parse one dense tensor dimension order.
    fn parse_tensor_dimension_order(&mut self) -> ParseResult<TensorDimensionOrder> {
        let token = self.eat_token(TokenType::Identifier)?;
        let order = match self.tree.source_text(token.span) {
            "rowMajor" => TensorDimensionOrder::RowMajor,
            "columnMajor" => TensorDimensionOrder::ColumnMajor,
            _ => return Err(ParseError::invalid("tensor dimension order", token.start)),
        };

        Ok(order)
    }

    /// Return a canonical field id for the provided field shape.
    fn intern_field(&mut self, field: Field, attributes: Vec<Attribute>) -> LocalNodeId<Field> {
        // reuse existing field
        let key = FieldKey::from_field(&field, &attributes);
        if let Some(existing) = self.field_intern.get(&key) {
            if !attributes.is_empty() {
                self.tree.set_attributes(*existing, attributes);
            }
            return *existing;
        }

        // insert a new field
        let field_id = self.tree.insert(field);
        if !attributes.is_empty() {
            self.tree.set_attributes(field_id, attributes);
        }
        self.field_intern.insert(key, field_id);
        field_id
    }

    /// Return a canonical type id for the provided type shape.
    pub(super) fn intern_type(&mut self, ty: Type) -> ParseResult<LocalNodeId<Type>> {
        // reuse existing type
        let key = TypeKey::from_type(&ty);
        if let Some(existing) = self.type_intern.get(&key) {
            return Ok(*existing);
        }

        // insert a new type
        let type_id = self.tree.insert_type(ty);
        self.type_intern.insert(key, type_id);

        Ok(type_id)
    }
}
