use destack_source::Span;

use crate::{
    FunctionTypeId, ParseError, ParseResult, Parser, ReferenceKind, Scalar, Space, TokenType,
    TypeId, ValueType, VectorType,
};

impl Parser<'_> {
    /// Parse one parenthesized logical value type list.
    pub(super) fn parse_value_types(&mut self) -> ParseResult<Vec<ValueType>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut types = Vec::new();

        // parse each logical value in source order
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            types.push(self.parse_value_type()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }

        Ok(types)
    }

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
            "functionPointer" => {
                let function_type = self.parse_function_type()?;

                Ok(ValueType::function_pointer(function_type))
            }
            "function" => {
                let function_type = self.parse_function_type()?;

                Ok(ValueType::function(function_type))
            }
            "slice" => {
                let (element, kind, space) = self.parse_slice_type()?;

                Ok(ValueType::slice(element, kind, space))
            }
            "dynamic" => {
                let constraint = self.parse_ty()?;

                Ok(ValueType::dynamic(constraint))
            }
            "tensor" => {
                let (scalar, ty) = self.parse_tensor_type()?;

                Ok(ValueType::tensor(scalar, ty))
            }
            "tensorView" => {
                self.eat_token(TokenType::LessThan)?;
                let scalar = self.parse_scalar_name()?;
                self.eat_token(TokenType::Comma)?;
                let ty = self.parse_type_name()?;
                self.eat_token(TokenType::GreaterThan)?;

                let ty = ValueType::tensor_view(scalar, ty);
                if !ty.is_defined() {
                    return Err(ParseError::new("invalid tensor view type", token.span));
                }

                Ok(ty)
            }
            "vector" => self.parse_vector(token.span),
            "words" => {
                self.eat_token(TokenType::LessThan)?;
                let word_count = self.parse_u16()?;
                self.eat_token(TokenType::GreaterThan)?;
                let ty = ValueType::words(word_count);
                if !ty.is_defined() {
                    return Err(ParseError::new("invalid word value type", token.span));
                }

                Ok(ty)
            }
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
        let element = self.parse_type_name()?;
        self.eat_token(TokenType::Comma)?;
        let kind = self.parse_reference_kind()?;
        self.eat_token(TokenType::Comma)?;
        let space = self.parse_space()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((element, kind, space))
    }

    /// Parse one tensor element representation and runtime type.
    fn parse_tensor_type(&mut self) -> ParseResult<(Scalar, TypeId)> {
        self.eat_token(TokenType::LessThan)?;
        let scalar = self.parse_scalar_name()?;
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_name()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok((scalar, ty))
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

    /// Parse one named type symbol inside angle brackets.
    fn parse_ty(&mut self) -> ParseResult<TypeId> {
        self.eat_token(TokenType::LessThan)?;
        let name = self.eat_token(TokenType::Identifier)?;
        let ty = self.symbols.types.get(self.text(name)).copied();
        let ty = ty.ok_or_else(|| ParseError::new("unknown type symbol", name.span))?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(ty)
    }

    /// Parse one type symbol name.
    pub(super) fn parse_type_name(&mut self) -> ParseResult<TypeId> {
        let token = self.eat_token(TokenType::Identifier)?;
        let text = self.text(token).to_string();
        if let Some(ty) = self.symbols.types.get(&text).copied() {
            return Ok(ty);
        }
        if Scalar::from_name(&text).is_none() {
            return Err(ParseError::new("unknown type", token.span));
        }

        let name = self.object.intern_string(&text);
        let ty = self.object.push_type(name);
        self.symbols.types.insert(text, ty);

        Ok(ty)
    }

    /// Parse one named function type inside angle brackets.
    fn parse_function_type(&mut self) -> ParseResult<FunctionTypeId> {
        self.eat_token(TokenType::LessThan)?;
        let name = self.eat_token(TokenType::Identifier)?;
        let function_type = self.symbols.function_types.get(self.text(name)).copied();
        let function_type =
            function_type.ok_or_else(|| ParseError::new("unknown function type", name.span))?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(function_type)
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
