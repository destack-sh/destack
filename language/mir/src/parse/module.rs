use crate::{
    Attribute, Global, GlobalInitializer, Linkage, LocalNodeId, Mutability, Type, TypeAlias, Value,
};

use super::constant::parse_string_literal;
use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Parse a type alias definition.
    pub(super) fn parse_type_alias(
        &mut self,
        attributes: Vec<Attribute>,
    ) -> ParseResult<LocalNodeId<TypeAlias>> {
        // alias header
        self.eat_token(TokenType::Type)?;
        self.eat_token(TokenType::At)?;

        // alias name
        let file_id = self.file_id;
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = Self::span_at(file_id, name_start, name.len());
        if self.type_alias_definitions.contains(&name) {
            return Err(ParseError::invalid(
                &format!("duplicate type alias '@{name}'"),
                name_start,
            ));
        }

        // resolve placeholder
        let placeholder_id = match self.type_alias_map.get(&name).copied() {
            Some(existing) => existing,
            None => {
                let placeholder = self.tree.insert_type(Type::Void);
                self.type_alias_map.insert(name.clone(), placeholder);
                placeholder
            }
        };

        // alias target type
        self.eat_token(TokenType::Equals)?;
        let ty = self.parse_type()?;

        // record alias
        let name_id = self.strings.intern(&name);
        let alias = TypeAlias {
            name: name_id,
            ty: placeholder_id,
        };
        let id = self.tree.insert(alias);
        self.tree.set_span(id, name_span);
        let metadata = self
            .tree
            .type_table
            .type_metadata_by_id
            .entry(placeholder_id)
            .or_default();
        metadata.name = Some(name_id);
        if ty != placeholder_id {
            let resolved = self.tree.get(ty).clone();
            *self.tree.get_mut(placeholder_id) = resolved;
        }
        self.type_alias_definitions.insert(name);

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse a global definition or declaration.
    /// Expect `[export|extern] global @name: type [= init] ; mut|const`.
    pub(super) fn parse_global(
        &mut self,
        linkage: Linkage,
        attributes: Vec<Attribute>,
    ) -> ParseResult<LocalNodeId<Global>> {
        // global header
        self.eat_token(TokenType::Global)?;
        self.eat_token(TokenType::At)?;

        // global name
        let file_id = self.file_id;
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = Self::span_at(file_id, name_start, name.len());

        // type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;

        // initializer
        let initializer = if linkage.is_import() {
            None
        } else {
            self.eat_token(TokenType::Equals)?;
            Some(self.parse_data_init()?)
        };

        // mutability annotation
        let mut mutability = Mutability::Immutable;
        if self.eat_token_maybe(TokenType::Semicolon) {
            if self.eat_token_maybe(TokenType::Mut) {
                mutability = Mutability::Mutable;
            } else if self.eat_token_maybe(TokenType::Const) {
                mutability = Mutability::Immutable;
            }
        }

        // record global
        let name_id = self.strings.intern(&name);
        let global = Global {
            name: name_id,
            ty,
            mutability,
            linkage,
            initializer,
        };
        let id = self.tree.insert(global);
        self.tree.set_span(id, name_span);
        self.global_map.insert(name, id);

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse a data initializer.
    fn parse_data_init(&mut self) -> ParseResult<GlobalInitializer> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("data initializer", self.pos()))?;

        match token.ty {
            // zero initializer
            TokenType::Identifier if token.text == "zeroinit" => {
                self.bump();
                Ok(GlobalInitializer::Zero)
            }
            // byte string literal
            TokenType::Identifier
                if token.text == "b"
                    && self
                        .peek_nth_token(1)
                        .is_some_and(|token| token.ty == TokenType::StringLiteral) =>
            {
                self.eat_token(TokenType::Identifier)?;
                let token = self.eat_token(TokenType::StringLiteral)?;
                let token_text = token.text.to_string();
                let token_start = token.start;
                let value = parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // string literal
            TokenType::StringLiteral => {
                let token_text = token.text.to_string();
                let token_start = token.start;
                self.bump();
                let value = parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::String(value))
            }
            // scalar constant
            TokenType::BoolLiteral
            | TokenType::IntLiteral
            | TokenType::FloatLiteral
            | TokenType::CharLiteral => {
                let constant = self.parse_constant()?;
                Ok(GlobalInitializer::Scalar(constant))
            }
            // aggregate initializer
            TokenType::OpenBrace => {
                self.bump();
                let mut elements = Vec::new();
                while !self.peek_token(TokenType::CloseBrace) {
                    elements.push(self.parse_data_init()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBrace)?;
                Ok(GlobalInitializer::Aggregate(elements))
            }
            _ => Err(ParseError::unexpected(
                "data initializer",
                token.ty,
                token.start,
            )),
        }
    }

    /// Record the type for a value in the current function.
    pub(super) fn record_value_type(&mut self, value: Value, ty: LocalNodeId<Type>) {
        if let Some(function_id) = self.current_function {
            let function = self.tree.get_mut(function_id);
            function.set_value_type(value, ty);
        }
    }
}
