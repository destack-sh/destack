use destack_source::Span;

use crate::{
    AddressSpace, Attribute, Copy, Field, FieldSpan, LocalNodeId, Mutability, ReferenceKind,
    TensorDimension, TensorLayout, Type, TypeDeclarationSpans, Value,
};

use super::error::{ParseError, ParseResult};
use super::key::{FieldKey, TypeKey};
use super::parser::Parser;
use super::token::TokenType;

impl Parser {
    /// Ensure the canonical hidden base type for one slice header.
    fn ensure_slice_data_type(
        &mut self,
        kind: ReferenceKind,
        element: LocalNodeId<Type>,
        mutability: Mutability,
        address_space: AddressSpace,
    ) {
        self.intern_type(Type::Reference {
            kind,
            address_space,
            mutability,
            pointee: element.into(),
            is_nullable: false,
        });
    }

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
                    let (kind, address_space, mutability) = self.parse_slice_qualifiers()?;
                    self.ensure_slice_data_type(kind, element, mutability, address_space.clone());
                    self.eat_token(TokenType::GreaterThan)?;
                    Type::Slice {
                        kind,
                        element: element.into(),
                        address_space,
                        mutability,
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
                            });
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
                    });
                    Type::FunctionPointer {
                        signature: signature.into(),
                    }
                } else if self.eat_token_maybe(TokenType::FatArrow) {
                    let result = self.parse_type()?;
                    let signature = self.intern_type(Type::FunctionSignature {
                        parameters,
                        result: result.into(),
                    });
                    self.tree.ensure_function_value_environment_type();
                    Type::Closure {
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
        let mut type_id = self.intern_type(ty);
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
                });
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
        let type_id = self.intern_type(struct_type);

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
        let (kind, address_space, mutability, pointee) = self.parse_reference_header()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Reference {
            kind,
            address_space,
            mutability,
            pointee: pointee.into(),
            is_nullable,
        })
    }

    /// Parse a tensor view type.
    fn parse_tensor_view_type(&mut self, is_nullable: bool) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (kind, address_space, mutability, element) = self.parse_reference_header()?;
        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let layout = self.parse_optional_tensor_layout()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::TensorView {
            kind,
            address_space,
            mutability,
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
    ) -> ParseResult<(ReferenceKind, AddressSpace, Mutability, LocalNodeId<Type>)> {
        let pointee = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;

        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("reference kind", self.pos()))?;
        let kind_text = self.tree.source_text(kind_token.span);
        let kind = match kind_token.ty {
            TokenType::Ownership | TokenType::Identifier => match kind_text {
                "managed" => ReferenceKind::Managed,
                "owned" => ReferenceKind::Owned,
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
        let mut mutability = Mutability::Mutable;

        while self.peek_token(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::OpenParen)
            {
                break;
            }

            self.bump();

            if self.eat_token_maybe(TokenType::Readonly) {
                mutability = Mutability::Immutable;
                continue;
            }

            if self.eat_token_maybe(TokenType::AddressSpace) {
                self.eat_token(TokenType::OpenParen)?;

                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("address space", self.pos()))?;
                let text = self.tree.source_text(token.span).to_string();
                address_space = match token.ty {
                    TokenType::Identifier | TokenType::Global | TokenType::Local => {
                        AddressSpace::from_name(&text)
                    }
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

        Ok((kind, address_space, mutability, pointee))
    }

    /// Parse optional trailing qualifiers for one slice type.
    fn parse_slice_qualifiers(&mut self) -> ParseResult<(ReferenceKind, AddressSpace, Mutability)> {
        let mut kind = ReferenceKind::Managed;
        let mut address_space = AddressSpace::Local;
        let mut mutability = Mutability::Mutable;

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
                    "managed" => kind = ReferenceKind::Managed,
                    "owned" => kind = ReferenceKind::Owned,
                    "borrowed" => kind = ReferenceKind::Borrowed,
                    "raw" => kind = ReferenceKind::Raw,
                    _ => {}
                }
                if matches!(qualifier_text, "managed" | "owned" | "borrowed" | "raw") {
                    self.bump();
                    continue;
                }
            }

            if self.eat_token_maybe(TokenType::Readonly) {
                mutability = Mutability::Immutable;
                continue;
            }

            if self.eat_token_maybe(TokenType::AddressSpace) {
                self.eat_token(TokenType::OpenParen)?;

                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("address space", self.pos()))?;
                let text = self.tree.source_text(token.span).to_string();
                address_space = match token.ty {
                    TokenType::Identifier | TokenType::Global | TokenType::Local => {
                        AddressSpace::from_name(&text)
                    }
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

        Ok((kind, address_space, mutability))
    }

    /// Parse an optional trailing tensor layout assignment.
    fn parse_optional_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        if self.eat_token_maybe(TokenType::Comma) {
            self.parse_tensor_layout_group()
        } else {
            Ok(TensorLayout::RowMajor)
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
                    if self.tree.source_text(ident.span) != "dynamic" {
                        return Err(ParseError::invalid("tensor shape dimension", ident.start));
                    }
                    shape.push(TensorDimension::Dynamic);
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
            "rowMajor" => Ok(TensorLayout::RowMajor),
            "columnMajor" => Ok(TensorLayout::ColumnMajor),
            "strided" => {
                self.eat_token(TokenType::OpenParen)?;
                let strides = self.parse_tensor_shape()?;
                self.eat_token(TokenType::CloseParen)?;
                Ok(TensorLayout::Strided { strides })
            }
            _ => Err(ParseError::invalid("tensor layout", token.start)),
        }
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
    pub(super) fn intern_type(&mut self, ty: Type) -> LocalNodeId<Type> {
        // reuse existing type
        let key = TypeKey::from_type(&ty);
        if let Some(existing) = self.type_intern.get(&key) {
            return *existing;
        }

        // insert a new type
        let type_id = self.tree.insert_type(ty);
        self.type_intern.insert(key, type_id);
        type_id
    }
}
