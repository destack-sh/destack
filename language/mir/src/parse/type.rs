use crate::{
    AddressSpace, Attribute, Copyability, Field, LocalNodeId, Mutability, ReferenceKind,
    TensorDimension, TensorLayout, Type, Value,
};

use super::error::{ParseError, ParseResult};
use super::key::{FieldKey, TypeKey};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Check if a token can start a type in a value position.
    pub(super) fn peek_type(&self, token_ty: TokenType) -> bool {
        matches!(
            token_ty,
            TokenType::Void
                | TokenType::Bool
                | TokenType::TypeName
                | TokenType::At
                | TokenType::Value
                | TokenType::Ref
                | TokenType::RefNullable
                | TokenType::TensorReference
                | TokenType::TensorReferenceNullable
                | TokenType::Tensor
                | TokenType::Vector
                | TokenType::Newtype
                | TokenType::OpenBracket
                | TokenType::OpenParen
                | TokenType::OpenBrace
                | TokenType::Fn
        )
    }

    /// Parse a type expression.
    pub(super) fn parse_type(&mut self) -> ParseResult<LocalNodeId<Type>> {
        let (token_ty, token_start, token_text) = {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("type", self.pos()))?;
            (token.ty, token.start, token.text)
        };

        // primitive or composite type
        let ty = match token_ty {
            TokenType::Void => {
                self.bump();
                Type::Void
            }
            TokenType::Bool => {
                self.bump();
                Type::Boolean
            }
            TokenType::Identifier | TokenType::TypeName => {
                if let Some(primitive) = self.parse_primitive_type(token_text) {
                    self.bump();
                    primitive
                } else {
                    return Err(ParseError::invalid("type", token_start));
                }
            }
            TokenType::At => {
                self.bump();
                let (name, _name_start) = self.parse_symbol_name()?;
                let alias_id = if let Some(existing) = self.type_alias_map.get(&name) {
                    *existing
                } else {
                    let placeholder = self.tree.insert_type(Type::Void);
                    self.type_alias_map.insert(name, placeholder);
                    placeholder
                };
                return Ok(alias_id);
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
            TokenType::TensorReference => self.parse_tensor_reference_type(false)?,
            TokenType::TensorReferenceNullable => self.parse_tensor_reference_type(true)?,
            TokenType::Tensor => self.parse_tensor_type()?,
            TokenType::Vector => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let token = self.eat_token(TokenType::IntLiteral)?;
                let token_text = token.text;
                let lanes = token_text.parse().map_err(|_| {
                    ParseError::invalid(&format!("vector lane count '{token_text}'"), token.start)
                })?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::Vector {
                    element,
                    lanes,
                    copyability: Copyability::default(),
                }
            }
            TokenType::Newtype => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let inner = self.parse_type()?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::Newtype {
                    inner,
                    copyability: Copyability::default(),
                }
            }
            TokenType::OpenBracket => {
                self.bump();
                let element = self.parse_type()?;
                self.eat_token(TokenType::Semicolon)?;
                let length = self.parse_int_literal()?;
                let length = u64::try_from(length)
                    .map_err(|_| ParseError::invalid("array length", self.pos()))?;
                self.eat_token(TokenType::CloseBracket)?;
                Type::Array {
                    element,
                    length,
                    copyability: Copyability::default(),
                }
            }
            TokenType::OpenParen => {
                self.bump();
                let mut elements = Vec::new();
                while !self.peek_token(TokenType::CloseParen) {
                    elements.push(self.parse_type()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseParen)?;
                Type::Tuple {
                    elements,
                    copyability: Copyability::default(),
                }
            }
            TokenType::Fn => {
                self.bump();
                self.eat_token(TokenType::OpenParen)?;
                let mut parameters = Vec::new();
                while !self.peek_token(TokenType::CloseParen) {
                    parameters.push(self.parse_type()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseParen)?;
                self.eat_token(TokenType::Arrow)?;
                let result = self.parse_type()?;
                Type::FunctionPointer { parameters, result }
            }
            TokenType::FnValue => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let signature = self.parse_type()?;
                self.eat_token(TokenType::GreaterThan)?;
                self.tree.ensure_function_value_environment_type();
                Type::FunctionValue { signature }
            }
            TokenType::OpenBrace => {
                self.bump();
                let mut fields = Vec::new();
                while !self.peek_token(TokenType::CloseBrace) {
                    // field attributes
                    let attributes = self.parse_attributes()?;

                    let mut name = None;
                    if self.peek_token(TokenType::At)
                        && self
                            .peek_nth_token(1)
                            .is_some_and(|token| token.ty == TokenType::Identifier)
                        && self
                            .peek_nth_token(2)
                            .is_some_and(|token| token.ty == TokenType::Colon)
                    {
                        // allow synthetic field names like @tag and @payload
                        self.eat_token(TokenType::At)?;
                        let name_text = {
                            let name_token = self.eat_token(TokenType::Identifier)?;
                            name_token.text.to_string()
                        };
                        self.eat_token(TokenType::Colon)?;
                        let name_text = format!("@{name_text}");
                        name = Some(self.strings.intern(&name_text));
                    } else if self.peek_token(TokenType::Identifier)
                        && let Some(next_token) = self.peek_nth_token(1)
                        && next_token.ty == TokenType::Colon
                    {
                        let name_text = {
                            let name_token = self.eat_token(TokenType::Identifier)?;
                            name_token.text.to_string()
                        };
                        self.eat_token(TokenType::Colon)?;
                        name = Some(self.strings.intern(&name_text));
                    }
                    let ty = self.parse_type()?;

                    let field = Field { name, ty };
                    fields.push(self.intern_field(field, attributes));
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBrace)?;
                let struct_type = Type::Struct {
                    fields,
                    copyability: Copyability::default(),
                };
                let type_id = self.intern_type(struct_type);
                self.record_layout_for_type(type_id)?;
                return Ok(type_id);
            }
            _ => {
                return Err(ParseError::unexpected("type", token_ty, token_start));
            }
        };
        let type_id = self.intern_type(ty);
        self.record_layout_for_type(type_id)?;
        Ok(type_id)
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
            pointee,
            is_nullable,
        })
    }

    /// Parse a tensor reference type.
    fn parse_tensor_reference_type(&mut self, is_nullable: bool) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (kind, address_space, mutability, element) = self.parse_reference_header()?;
        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_tensor_shape()?;
        let layout = self.parse_optional_tensor_layout()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::TensorReference {
            kind,
            address_space,
            mutability,
            element,
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
            element,
            shape,
            layout,
            copyability: Copyability::default(),
        })
    }

    /// Parse the reference header for ref and tensor_ref types.
    fn parse_reference_header(
        &mut self,
    ) -> ParseResult<(ReferenceKind, AddressSpace, Mutability, LocalNodeId<Type>)> {
        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("reference kind", self.pos()))?;
        let kind_text = kind_token.text;
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

        // address space
        let address_space = if self.eat_token_maybe(TokenType::AddrSpace) {
            self.eat_token(TokenType::OpenParen)?;

            // address space name or id
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("address space", self.pos()))?;
            let token_start = token.start;
            let text = token.text.to_string();
            let address_space = match token.ty {
                TokenType::Identifier | TokenType::Global | TokenType::Local => match text.as_str()
                {
                    "generic" => AddressSpace::Generic,
                    "stack" => AddressSpace::Stack,
                    "shared" => AddressSpace::Shared,
                    "local" => AddressSpace::Local,
                    "global" => AddressSpace::Global,
                    "constant" => AddressSpace::Constant,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("address space '{text}'"),
                            token_start,
                        ));
                    }
                },
                TokenType::IntLiteral => {
                    let id: u32 = text
                        .parse()
                        .map_err(|_| ParseError::invalid("address space", token_start))?;
                    AddressSpace::Target(id)
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
            address_space
        } else {
            AddressSpace::Generic
        };

        // mutability
        let mutability = if self.eat_token_maybe(TokenType::Readonly)
            || self.eat_token_maybe(TokenType::Const)
        {
            Mutability::Immutable
        } else {
            Mutability::Mutable
        };

        let pointee = self.parse_type()?;
        Ok((kind, address_space, mutability, pointee))
    }

    /// Parse an optional trailing tensor layout assignment.
    fn parse_optional_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        if self.eat_token_maybe(TokenType::Comma) {
            self.parse_tensor_layout_assignment()
        } else {
            Ok(TensorLayout::RowMajor)
        }
    }

    /// Parse a tensor shape list.
    fn parse_tensor_shape(&mut self) -> ParseResult<Vec<TensorDimension>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut shape = Vec::new();
        while !self.peek_token(TokenType::CloseBracket) {
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
                    if ident.text != "dynamic" {
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
        self.eat_token(TokenType::CloseBracket)?;
        Ok(shape)
    }

    /// Parse a tensor layout assignment.
    fn parse_tensor_layout_assignment(&mut self) -> ParseResult<TensorLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != "layout" {
            return Err(ParseError::invalid("layout assignment", token.start));
        }
        self.eat_token(TokenType::Equals)?;
        self.parse_tensor_layout()
    }

    /// Parse a tensor layout specifier.
    fn parse_tensor_layout(&mut self) -> ParseResult<TensorLayout> {
        let token = self.eat_token(TokenType::Identifier)?;
        match token.text {
            "row_major" => Ok(TensorLayout::RowMajor),
            "column_major" => Ok(TensorLayout::ColumnMajor),
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
