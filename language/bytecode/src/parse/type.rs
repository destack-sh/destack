use destack_source::Span;

use crate::{
    LayoutId, ParseError, ParseResult, Parser, ReferenceKind, ReferenceType, Scalar, Space,
    TokenType, TypeId, ValueType, VectorType,
};

impl Parser<'_> {
    /// Parse one logical bytecode value type.
    pub(super) fn parse_value_type(&mut self) -> ParseResult<ValueType> {
        let token = self.eat_token(TokenType::Identifier)?;
        let name = self.text(token);

        if let Some(scalar) = Scalar::from_name(name) {
            return Ok(ValueType::scalar(scalar));
        }

        match name {
            "int128" => Ok(ValueType::int128()),
            "uint128" => Ok(ValueType::uint128()),
            "typeId" => Ok(ValueType::type_id()),
            "pointer" => Ok(ValueType::pointer()),
            "ref" => {
                let (kind, space) = self.parse_reference_qualifiers()?;

                Ok(ValueType::reference(kind, space))
            }
            "uninit" => self.parse_uninit_type(),
            "fn" => Ok(ValueType::function_pointer()),
            "function" => {
                let (kind, space) = self.parse_reference_qualifiers()?;

                Ok(ValueType::function(ReferenceType::new(kind, space)))
            }
            "slice" => {
                let (element, kind, space) = self.parse_slice_type()?;

                Ok(ValueType::slice(element, kind, space))
            }
            "dynamic" => {
                self.eat_token(TokenType::LessThan)?;
                let constraint = self.parse_type_id()?;
                self.eat_token(TokenType::Comma)?;
                let space = self.parse_space()?;
                self.eat_token(TokenType::GreaterThan)?;

                Ok(ValueType::dynamic(constraint, space))
            }
            "tensor" => {
                let (ty, scalar, space) = self.parse_tensor_type()?;

                Ok(ValueType::tensor(scalar, ty, space))
            }
            "tensorView" => {
                self.eat_token(TokenType::LessThan)?;
                let ty = self.parse_type_id()?;
                self.eat_token(TokenType::Comma)?;
                let scalar = self.parse_scalar_name()?;
                self.eat_token(TokenType::Comma)?;
                let kind = self.parse_reference_kind()?;
                self.eat_token(TokenType::Comma)?;
                let space = self.parse_space()?;
                self.eat_token(TokenType::Comma)?;
                let word_count = self.parse_u16()?;
                self.eat_token(TokenType::GreaterThan)?;

                let reference = ReferenceType::new(kind, space);
                let ty = ValueType::tensor_view(scalar, ty, reference, word_count);
                if !ty.is_defined() {
                    return Err(ParseError::new("invalid tensor view type", token.span));
                }

                Ok(ty)
            }
            "vector" => self.parse_vector(token.span),
            _ => Err(ParseError::new("expected bytecode value type", token.span)),
        }
    }

    /// Parse one uninitialized reference or slice type.
    fn parse_uninit_type(&mut self) -> ParseResult<ValueType> {
        self.eat_token(TokenType::LessThan)?;
        let constructor = self.eat_token(TokenType::Identifier)?;
        let ty = match self.text(constructor) {
            "ref" => {
                let (kind, space) = self.parse_reference_qualifiers()?;

                ValueType::uninit_reference(kind, space)
            }
            "slice" => {
                let (element, kind, space) = self.parse_slice_type()?;

                ValueType::uninit_slice(element, kind, space)
            }
            _ => {
                return Err(ParseError::new(
                    "expected reference or slice type",
                    constructor.span,
                ));
            }
        };
        self.eat_token(TokenType::GreaterThan)?;

        Ok(ty)
    }

    /// Parse one slice element, ownership, and space argument list.
    fn parse_slice_type(&mut self) -> ParseResult<(TypeId, ReferenceKind, Space)> {
        self.eat_token(TokenType::LessThan)?;
        let element = self.parse_type_id()?;
        self.eat_token(TokenType::Comma)?;
        let kind = self.parse_reference_kind()?;
        self.eat_token(TokenType::Comma)?;
        let space = self.parse_space()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((element, kind, space))
    }

    /// Parse one tensor element representation and runtime type.
    fn parse_tensor_type(&mut self) -> ParseResult<(TypeId, Scalar, Space)> {
        self.eat_token(TokenType::LessThan)?;
        let ty = self.parse_type_id()?;
        self.eat_token(TokenType::Comma)?;
        let scalar = self.parse_scalar_name()?;
        self.eat_token(TokenType::Comma)?;
        let space = self.parse_space()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((ty, scalar, space))
    }

    /// Parse one reference ownership and space argument list.
    pub(super) fn parse_reference_qualifiers(&mut self) -> ParseResult<(ReferenceKind, Space)> {
        self.eat_token(TokenType::LessThan)?;
        let kind = self.parse_reference_kind()?;
        self.eat_token(TokenType::Comma)?;
        let space = self.parse_space()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((kind, space))
    }

    /// Parse one reference ownership name.
    fn parse_reference_kind(&mut self) -> ParseResult<ReferenceKind> {
        let kind = self.eat_token(TokenType::Identifier)?;

        ReferenceKind::from_name(self.text(kind))
            .ok_or_else(|| ParseError::new("expected reference kind", kind.span))
    }

    /// Parse one `space(local)` or `space(shared)` argument.
    fn parse_space(&mut self) -> ParseResult<Space> {
        self.eat_name("space")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let space = self.eat_token(TokenType::Identifier)?;
        let space = Space::from_name(self.text(space))
            .ok_or_else(|| ParseError::new("expected local or shared space", space.span))?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(space)
    }

    /// Parse one object-local type id.
    pub(super) fn parse_type_id(&mut self) -> ParseResult<TypeId> {
        let token = self.eat_token(TokenType::Identifier)?;
        let ty = TypeId::from_name(self.text(token))
            .ok_or_else(|| ParseError::new("expected type id", token.span))?;

        Ok(ty)
    }

    /// Parse one object-local layout id.
    pub(super) fn parse_layout_id(&mut self) -> ParseResult<LayoutId> {
        let token = self.eat_token(TokenType::Identifier)?;
        let layout = LayoutId::from_name(self.text(token))
            .ok_or_else(|| ParseError::new("expected layout id", token.span))?;

        Ok(layout)
    }

    /// Parse one fixed-width vector type.
    fn parse_vector(&mut self, span: Span) -> ParseResult<ValueType> {
        self.eat_token(TokenType::LessThan)?;
        let scalar = self.parse_scalar_name()?;
        self.eat_token(TokenType::Comma)?;
        let lane_count = self.parse_u16()?;
        self.eat_token(TokenType::GreaterThan)?;
        let vector = VectorType::new(scalar, lane_count);
        if lane_count == 0 {
            return Err(ParseError::new("vector requires at least one lane", span));
        }

        Ok(ValueType::vector(vector))
    }
}
