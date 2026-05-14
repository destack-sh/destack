use destack_source::Span;

use crate::{
    Access, AddressSpace, Attribute, Copy, Field, FieldSpan, Lifetime, LifetimeOrigin, LocalNodeId,
    ReferenceKind, TensorDimension, TensorDimensionOrder, TensorLayout, TensorStride, Type,
    TypeDeclarationSpans, UnionVariant, Value,
};

use super::error::{ParseError, ParseResult};
use super::key::{FieldKey, TypeKey};
use super::parser::Parser;
use super::token::TokenType;

impl Parser {
    /// Parse a type expression and return its enclosing span.
    pub(super) fn parse_type_part(&mut self) -> ParseResult<(LocalNodeId<Type>, Span)> {
        let type_start = self.pos();
        let ty = self.parse_type()?;
        let span = self.span_from_parse_start(type_start);

        Ok((ty, span))
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
    pub(super) fn peek_type(&self, token_ty: TokenType) -> bool {
        matches!(
            token_ty,
            TokenType::Void
                | TokenType::Boolean
                | TokenType::TypeName
                | TokenType::Value
                | TokenType::Ref
                | TokenType::RefNullable
                | TokenType::TensorView
                | TokenType::TensorViewNullable
                | TokenType::Tensor
                | TokenType::Vector
                | TokenType::Newtype
                | TokenType::OpenParen
                | TokenType::OpenBrace
        )
    }

    /// Parse a type expression.
    pub(super) fn parse_type(&mut self) -> ParseResult<LocalNodeId<Type>> {
        let (token_ty, token_start, token_text) = {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("type", self.pos()))?;
            (
                token.ty,
                token.start,
                self.tree.source_text(token.span).to_string(),
            )
        };

        // primitive or composite type
        let ty = match token_ty {
            TokenType::Void => {
                self.bump();
                Type::Void
            }
            TokenType::Boolean => {
                self.bump();
                Type::Boolean
            }
            TokenType::Identifier | TokenType::TypeName => {
                if let Some(primitive) = self.parse_primitive_type(&token_text) {
                    self.bump();
                    primitive
                } else if token_text == "slice" {
                    self.bump();
                    self.eat_token(TokenType::LessThan)?;
                    let element = self.parse_type()?;
                    let (kind, lifetime, address_space, access) = self.parse_slice_qualifiers()?;
                    self.eat_token(TokenType::GreaterThan)?;
                    Type::Slice {
                        kind,
                        lifetime,
                        element: element.into(),
                        address_space,
                        access,
                    }
                } else if token_text == "atomic" {
                    self.bump();
                    self.eat_token(TokenType::LessThan)?;
                    let value = self.parse_type()?;
                    self.eat_token(TokenType::GreaterThan)?;
                    Type::Atomic {
                        value: value.into(),
                    }
                } else if token_text == "any" {
                    self.bump();
                    self.eat_token(TokenType::LessThan)?;
                    let interface = self.parse_type()?;
                    self.eat_token(TokenType::GreaterThan)?;
                    Type::Any {
                        interface: interface.into(),
                    }
                } else if token_text == "union" {
                    self.bump();
                    self.eat_token(TokenType::LessThan)?;
                    let tag = self.parse_type()?.into();
                    self.eat_token(TokenType::Semicolon)?;
                    let mut variants = Vec::new();
                    while !self.peek_token(TokenType::GreaterThan) {
                        let tag_value = self.parse_int_literal()?;
                        if tag_value < 0 {
                            return Err(ParseError::invalid("negative union tag", self.pos()));
                        }
                        self.eat_token(TokenType::Colon)?;
                        let ty = self.parse_type()?.into();
                        variants.push(UnionVariant {
                            tag: tag_value as u64,
                            ty,
                        });
                        if !self.eat_token_maybe(TokenType::Comma) {
                            break;
                        }
                    }
                    self.eat_token(TokenType::GreaterThan)?;
                    Type::Union {
                        tag,
                        variants,
                        copy: Copy::default(),
                    }
                } else if let Some(alias_id) = self.type_alias_map.get(&token_text).copied() {
                    self.bump();

                    // alias postfixes
                    let mut type_id = alias_id;
                    while self.eat_token_maybe(TokenType::OpenBracket) {
                        if self.eat_token_maybe(TokenType::CloseBracket) {
                            return Err(ParseError::invalid("dynamic array type", self.pos()));
                        } else {
                            let length = self.parse_int_literal()?;
                            let length = u64::try_from(length)
                                .map_err(|_| ParseError::invalid("array length", self.pos()))?;
                            self.eat_token(TokenType::CloseBracket)?;

                            type_id = self.intern_type(Type::Array {
                                element: type_id.into(),
                                length,
                                copy: Copy::default(),
                            })?;
                        }
                    }

                    return Ok(type_id);
                } else {
                    return Err(ParseError::invalid("type", token_start));
                }
            }
            TokenType::Value => {
                self.bump();
                let value_id: u32 = token_text[1..].parse().map_err(|_| {
                    ParseError::invalid(&format!("value id '{token_text}'"), token_start)
                })?;
                let value = Value(value_id);
                let ty = self.current_function.and_then(|function_id| {
                    let function = self.tree.get(function_id);
                    function.value_type(value)
                });
                return ty.ok_or_else(|| {
                    ParseError::invalid(&format!("value type '{token_text}'"), token_start)
                });
            }
            TokenType::Ref | TokenType::RefNullable => {
                self.parse_reference_type(token_ty == TokenType::RefNullable)?
            }
            TokenType::TensorView => self.parse_tensor_view_type(false)?,
            TokenType::TensorViewNullable => self.parse_tensor_view_type(true)?,
            TokenType::Tensor => self.parse_tensor_type()?,
            TokenType::Vector => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let token = self.eat_token(TokenType::IntLiteral)?;
                let token_text = self.tree.source_text(token.span).to_string();
                let lanes = token_text.parse().map_err(|_| {
                    ParseError::invalid(&format!("vector lane count '{token_text}'"), token.start)
                })?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::Vector {
                    element: element.into(),
                    lanes,
                    copy: Copy::default(),
                }
            }
            TokenType::Newtype => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let inner = self.parse_type()?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::Newtype {
                    inner: inner.into(),
                    copy: Copy::default(),
                }
            }
            TokenType::OpenParen => {
                self.bump();
                let mut parameters = Vec::new();
                while !self.peek_token(TokenType::CloseParen) {
                    parameters.push(self.parse_type()?.into());
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseParen)?;

                if self.eat_token_maybe(TokenType::Arrow) {
                    let result = self.parse_type()?;
                    let signature = self.intern_type(Type::FunctionSignature {
                        parameters,
                        result: result.into(),
                    })?;
                    Type::FunctionPointer {
                        signature: signature.into(),
                    }
                } else if self.eat_token_maybe(TokenType::FatArrow) {
                    let result = self.parse_type()?;
                    let signature = self.intern_type(Type::FunctionSignature {
                        parameters,
                        result: result.into(),
                    })?;
                    self.tree.ensure_callable_environment_type();
                    Type::Callable {
                        signature: signature.into(),
                    }
                } else {
                    Type::Tuple {
                        elements: parameters,
                        copy: Copy::default(),
                    }
                }
            }
            TokenType::OpenBrace => {
                let (type_id, _, _) = self.parse_struct_type()?;
                return Ok(type_id);
            }
            _ => {
                return Err(ParseError::unexpected("type", token_ty, token_start));
            }
        };

        // intern the base type first so postfix array syntax can wrap it
        let mut type_id = self.intern_type(ty)?;
        // parse postfix array suffixes like `int32[4]`
        while self.eat_token_maybe(TokenType::OpenBracket) {
            if self.eat_token_maybe(TokenType::CloseBracket) {
                return Err(ParseError::invalid("dynamic array type", self.pos()));
            } else {
                let length = self.parse_int_literal()?;
                let length = u64::try_from(length)
                    .map_err(|_| ParseError::invalid("array length", self.pos()))?;
                self.eat_token(TokenType::CloseBracket)?;

                type_id = self.intern_type(Type::Array {
                    element: type_id.into(),
                    length,
                    copy: Copy::default(),
                })?;
            }
        }

        Ok(type_id)
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
            if self.peek_token(TokenType::At)
                && self
                    .peek_nth_token(1)
                    .is_some_and(|token| token.ty == TokenType::Identifier)
                && self
                    .peek_nth_token(2)
                    .is_some_and(|token| token.ty == TokenType::Colon)
            {
                self.eat_token(TokenType::At)?;
                let name_token = self.eat_token(TokenType::Identifier)?;
                let name_start = name_token.start;
                let name_text = self.tree.source_text(name_token.span).to_string();
                let name_length = name_text.len() + 1;
                let display_name = format!("@{name_text}");
                let span = self.span_at(name_start, name_length);
                self.eat_token(TokenType::Colon)?;
                name = Some(self.strings.intern(&display_name));
                name_span = Some(span);
            } else if self.peek_token(TokenType::Identifier)
                && let Some(next_token) = self.peek_nth_token(1)
                && next_token.ty == TokenType::Colon
            {
                let name_token = self.eat_token(TokenType::Identifier)?;
                let name_start = name_token.start;
                let name_text = self.tree.source_text(name_token.span).to_string();
                let name_length = name_text.len();
                let span = self.span_at(name_start, name_length);
                self.eat_token(TokenType::Colon)?;
                name = Some(self.strings.intern(&name_text));
                name_span = Some(span);
            }

            // field type
            let (ty, type_span) = self.parse_type_part()?;

            // field node
            let field = Field {
                name,
                ty: ty.into(),
            };
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
    fn parse_reference_type(&mut self, is_nullable: bool) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (kind, lifetime, address_space, access, pointee) = self.parse_reference_header()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Reference {
            kind,
            lifetime,
            address_space,
            access,
            pointee: pointee.into(),
            is_nullable,
        })
    }

    /// Parse a tensor view type.
    fn parse_tensor_view_type(&mut self, is_nullable: bool) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (kind, lifetime, address_space, access, element) = self.parse_reference_header()?;
        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let layout = self.parse_optional_tensor_layout()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::TensorView {
            kind,
            lifetime,
            address_space,
            access,
            element: element.into(),
            shape,
            layout,
            is_nullable,
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

    /// Parse the reference header for ref, slice, and tensorView types.
    fn parse_reference_header(
        &mut self,
    ) -> ParseResult<(
        ReferenceKind,
        Lifetime,
        AddressSpace,
        Access,
        LocalNodeId<Type>,
    )> {
        let pointee = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;

        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("reference kind", self.pos()))?;
        let kind_text = self.tree.source_text(kind_token.span);
        let kind = match kind_token.ty {
            TokenType::Ownership | TokenType::Identifier => match kind_text {
                "managed" => ReferenceKind::Managed,
                "unique" => ReferenceKind::Unique,
                "borrowed" => ReferenceKind::Borrowed,
                "raw" => ReferenceKind::Raw,
                _ => {
                    return Err(ParseError::invalid(
                        &format!("reference kind '{kind_text}'"),
                        kind_token.start,
                    ));
                }
            },
            _ => {
                return Err(ParseError::unexpected(
                    "reference kind",
                    kind_token.ty,
                    kind_token.start,
                ));
            }
        };
        self.bump();

        let mut address_space = AddressSpace::Local;
        let mut access = Access::Mutable;
        let mut lifetime = Lifetime::empty();

        while self.peek_token(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::OpenParen)
            {
                break;
            }

            self.bump();

            if self.eat_token_maybe(TokenType::Readonly) {
                access = Access::Readonly;
                continue;
            }

            if self.eat_identifier_text("exclusive") {
                access = Access::Exclusive;
                continue;
            }

            if self.eat_identifier_text("lifetime") {
                lifetime = self.parse_lifetime_group()?;
                continue;
            }

            if self.eat_token_maybe(TokenType::AddressSpace) {
                self.eat_token(TokenType::OpenParen)?;

                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("address space", self.pos()))?;
                let text = self.tree.source_text(token.span).to_string();
                address_space = match token.ty {
                    TokenType::Identifier | TokenType::Local => AddressSpace::from_name(&text),
                    _ => {
                        return Err(ParseError::unexpected(
                            "address space",
                            token.ty,
                            token.start,
                        ));
                    }
                };
                self.bump();
                self.eat_token(TokenType::CloseParen)?;
                continue;
            }

            return Err(ParseError::invalid("reference qualifier", self.pos()));
        }

        if kind != ReferenceKind::Borrowed && !lifetime.is_empty() {
            return Err(ParseError::invalid("borrowed lifetime", self.pos()));
        }

        Ok((kind, lifetime, address_space, access, pointee))
    }

    /// Parse optional trailing qualifiers for one slice type.
    fn parse_slice_qualifiers(
        &mut self,
    ) -> ParseResult<(ReferenceKind, Lifetime, AddressSpace, Access)> {
        let mut kind = None;
        let mut address_space = AddressSpace::Local;
        let mut access = Access::Mutable;
        let mut lifetime = Lifetime::empty();

        while self.peek_token(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::OpenParen)
            {
                break;
            }

            self.bump();

            let qualifier = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("slice qualifier", self.pos()))?;
            let qualifier_text = self.tree.source_text(qualifier.span);

            if matches!(qualifier.ty, TokenType::Ownership | TokenType::Identifier) {
                match qualifier_text {
                    "managed" => kind = Some(ReferenceKind::Managed),
                    "unique" => kind = Some(ReferenceKind::Unique),
                    "borrowed" => kind = Some(ReferenceKind::Borrowed),
                    "raw" => kind = Some(ReferenceKind::Raw),
                    _ => {}
                }
                if matches!(qualifier_text, "managed" | "unique" | "borrowed" | "raw") {
                    self.bump();
                    continue;
                }
            }

            if self.eat_token_maybe(TokenType::Readonly) {
                access = Access::Readonly;
                continue;
            }

            if self.eat_identifier_text("exclusive") {
                access = Access::Exclusive;
                continue;
            }

            if self.eat_identifier_text("lifetime") {
                lifetime = self.parse_lifetime_group()?;
                continue;
            }

            if self.eat_token_maybe(TokenType::AddressSpace) {
                self.eat_token(TokenType::OpenParen)?;

                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("address space", self.pos()))?;
                let text = self.tree.source_text(token.span).to_string();
                address_space = match token.ty {
                    TokenType::Identifier | TokenType::Local => AddressSpace::from_name(&text),
                    _ => {
                        return Err(ParseError::unexpected(
                            "address space",
                            token.ty,
                            token.start,
                        ));
                    }
                };
                self.bump();
                self.eat_token(TokenType::CloseParen)?;
                continue;
            }

            return Err(ParseError::invalid("slice qualifier", self.pos()));
        }

        let kind = kind.ok_or_else(|| ParseError::invalid("slice kind", self.pos()))?;
        if kind != ReferenceKind::Borrowed && !lifetime.is_empty() {
            return Err(ParseError::invalid("borrowed lifetime", self.pos()));
        }

        Ok((kind, lifetime, address_space, access))
    }

    /// Parse a lifetime qualifier group.
    fn parse_lifetime_group(&mut self) -> ParseResult<Lifetime> {
        self.eat_token(TokenType::OpenParen)?;
        let mut origins = Vec::new();

        while !self.peek_token(TokenType::CloseParen) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("lifetime", self.pos()))?;
            match token.ty {
                TokenType::Identifier if self.tree.source_text(token.span) == "static" => {
                    origins.push(LifetimeOrigin::Static);
                    self.bump();
                }
                TokenType::IntLiteral => {
                    let index = self.parse_int_literal()?;
                    let index = u32::try_from(index)
                        .map_err(|_| ParseError::invalid("lifetime parameter", self.pos()))?;
                    origins.push(LifetimeOrigin::Parameter(index));
                }
                _ => {
                    return Err(ParseError::unexpected(
                        "lifetime origin",
                        token.ty,
                        token.start,
                    ));
                }
            }

            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParen)?;
        if origins.is_empty() {
            return Err(ParseError::invalid("lifetime origin", self.pos()));
        }

        Ok(Lifetime::new(origins))
    }

    /// Parse an optional trailing tensor layout assignment.
    fn parse_optional_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        if self.eat_token_maybe(TokenType::Comma) {
            self.parse_tensor_layout_group()
        } else {
            Ok(TensorLayout::Dense {
                order: TensorDimensionOrder::RowMajor,
            })
        }
    }

    /// Parse a tensor shape list.
    fn parse_tensor_shape(&mut self) -> ParseResult<Vec<TensorDimension>> {
        self.eat_token(TokenType::OpenParen)?;
        let mut shape = Vec::new();
        while !self.peek_token(TokenType::CloseParen) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("tensor shape", self.pos()))?;
            match token.ty {
                TokenType::IntLiteral => {
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
                        token.ty,
                        token.start,
                    ));
                }
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParen)?;
        Ok(shape)
    }

    /// Parse a grouped tensor layout clause.
    fn parse_tensor_layout_group(&mut self) -> ParseResult<TensorLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        if self.tree.source_text(token.span) != "layout" {
            return Err(ParseError::invalid("layout group", token.start));
        }
        self.eat_token(TokenType::OpenParen)?;
        let layout = self.parse_tensor_layout()?;
        self.eat_token(TokenType::CloseParen)?;
        Ok(layout)
    }

    /// Parse a tensor layout specifier.
    fn parse_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        match self.tree.source_text(token.span) {
            "dense" => {
                self.eat_token(TokenType::OpenParen)?;
                let order = self.parse_tensor_dimension_order()?;
                self.eat_token(TokenType::CloseParen)?;
                Ok(TensorLayout::Dense { order })
            }
            "strided" => {
                self.eat_token(TokenType::OpenParen)?;
                let strides = self.parse_tensor_strides()?;
                self.eat_token(TokenType::CloseParen)?;
                Ok(TensorLayout::Strided { strides })
            }
            "backend" => {
                self.eat_token(TokenType::OpenParen)?;
                let name = self.eat_token(TokenType::Identifier)?;
                let name = self.tree.source_text(name.span).to_string();
                self.eat_token(TokenType::CloseParen)?;
                Ok(TensorLayout::Backend { name })
            }
            _ => Err(ParseError::invalid("tensor layout", token.start)),
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

    /// Parse a tensor stride list.
    fn parse_tensor_strides(&mut self) -> ParseResult<Vec<TensorStride>> {
        self.eat_token(TokenType::OpenParen)?;
        let mut strides = Vec::new();
        while !self.peek_token(TokenType::CloseParen) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("tensor strides", self.pos()))?;
            match token.ty {
                TokenType::IntLiteral => {
                    let stride = self.parse_int_literal()?;
                    let stride = i64::try_from(stride)
                        .map_err(|_| ParseError::invalid("tensor stride", self.pos()))?;
                    strides.push(TensorStride::Static(stride));
                }
                TokenType::Identifier => {
                    let ident = self.eat_token(TokenType::Identifier)?;
                    let text = self.tree.source_text(ident.span);
                    if text == "dynamic" {
                        strides.push(TensorStride::Dynamic);
                    } else {
                        strides.push(TensorStride::Symbol(text.to_string()));
                    }
                }
                _ => {
                    return Err(ParseError::unexpected(
                        "tensor stride",
                        token.ty,
                        token.start,
                    ));
                }
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParen)?;
        Ok(strides)
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
