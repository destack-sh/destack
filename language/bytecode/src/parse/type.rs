use destack_source::Span;

use crate::{
    LayoutId, ParseError, ParseResult, Parser, ReferenceKind, ReferenceType, Scalar, Storage,
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
                let (kind, storage) = self.parse_reference_qualifiers()?;

                Ok(ValueType::reference(kind, storage))
            }
            "uninit" => self.parse_uninit_type(),
            "fn" => Ok(ValueType::function_pointer()),
            "function" => {
                let (kind, storage) = self.parse_reference_qualifiers()?;

                Ok(ValueType::function(ReferenceType::new(kind, storage)))
            }
            "slice" => {
                let (element, kind, storage) = self.parse_slice_type()?;

                Ok(ValueType::slice(element, kind, storage))
            }
            "dynamic" => {
                self.eat_token(TokenType::LessThan)?;
                let constraint = self.parse_type_id()?;
                self.eat_token(TokenType::Comma)?;
                let kind = self.parse_reference_kind()?;
                self.eat_token(TokenType::Comma)?;
                let storage = self.parse_storage()?;
                self.eat_token(TokenType::GreaterThan)?;

                let reference = ReferenceType::new(kind, storage);

                Ok(ValueType::dynamic(constraint, reference))
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
                let (kind, storage) = self.parse_reference_qualifiers()?;

                ValueType::uninit_reference(kind, storage)
            }
            "slice" => {
                let (element, kind, storage) = self.parse_slice_type()?;

                ValueType::uninit_slice(element, kind, storage)
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

    /// Parse one slice element, ownership, and storage argument list.
    fn parse_slice_type(&mut self) -> ParseResult<(TypeId, ReferenceKind, Storage)> {
        self.eat_token(TokenType::LessThan)?;
        let element = self.parse_type_id()?;
        self.eat_token(TokenType::Comma)?;
        let kind = self.parse_reference_kind()?;
        self.eat_token(TokenType::Comma)?;
        let storage = self.parse_storage()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((element, kind, storage))
    }

    /// Parse one reference ownership and storage argument list.
    pub(super) fn parse_reference_qualifiers(&mut self) -> ParseResult<(ReferenceKind, Storage)> {
        self.eat_token(TokenType::LessThan)?;
        let kind = self.parse_reference_kind()?;
        self.eat_token(TokenType::Comma)?;
        let storage = self.parse_storage()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((kind, storage))
    }

    /// Parse one reference ownership name.
    pub(super) fn parse_reference_kind(&mut self) -> ParseResult<ReferenceKind> {
        let kind = self.eat_token(TokenType::Identifier)?;

        ReferenceKind::from_name(self.text(kind))
            .ok_or_else(|| ParseError::new("expected reference kind", kind.span))
    }

    /// Parse one reference storage.
    pub(super) fn parse_storage(&mut self) -> ParseResult<Storage> {
        let token = self.eat_token(TokenType::Identifier)?;
        let mut name = self.text(token);
        if name == "shared" && self.peek_name("global") {
            self.eat_name("global")?;
            name = "shared global";
        }

        Storage::from_name(name)
            .ok_or_else(|| ParseError::new("expected reference storage", token.span))
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
