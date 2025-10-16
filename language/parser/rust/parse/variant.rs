#![allow(clippy::type_complexity)]

use dyst_ast::Expression;

use crate::TokenType;
use crate::parse::prelude::*;

use crate::{NodeId, NodeType, Parser, ParserError, ParserResult, VariantField};

static VARIANT_FIELD_START_TOKENS: [TokenType; 6] = [
    TokenType::Colon,
    TokenType::Assign,
    TokenType::Newline,
    TokenType::Comma,
    TokenType::Semicolon,
    TokenType::CloseBrace,
];

impl<'a> Parser<'a> {
    /// Eat a variant body (without the parenthesis, without any expressions).
    pub fn eat_variant_body(&mut self) -> ParserResult<Vec<NodeId<VariantField>>> {
        let mut tuple_fields: Vec<NodeId<VariantField>> = Vec::new();
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok()
                || self.peek_token(TokenType::CloseBrace).is_ok()
            {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating tuple fields
            else {
                let field = self.eat_variant_field()?;
                tuple_fields.push(field);
            }
        }
        Ok(tuple_fields)
    }

    /// Peek a variant field: `name: Type` with optional default `= <expr>`.
    pub(crate) fn peek_variant_field(&self) -> ParserResult<()> {
        if (self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token_in(&VARIANT_FIELD_START_TOKENS).is_ok())
            || (self.peek_visibility().is_ok()
                && self.peek_next_token(TokenType::Identifier).is_ok()
                && self
                    .peek_next_next_token_in(&VARIANT_FIELD_START_TOKENS)
                    .is_ok())
        {
            Ok(())
        } else {
            Err(ParserError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a single variant field: `T`,`name: T`, or `name: T = <expr>` (including visibility).
    ///
    /// Examples:
    /// ```
    /// T
    /// name: T
    /// name: T = <expr>
    /// public T
    /// ```
    pub(crate) fn eat_variant_field(&mut self) -> ParserResult<NodeId<VariantField>> {
        let start = self.mark();

        // visibility
        let visibility = self.eat_visibility_maybe()?;

        // name:
        let name =
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let name = self.eat_identifier()?;
                self.eat_colon()?;
                Some(name)
            } else {
                None
            };

        // type
        let ty = self
            .with_options(self.options.in_type(), |parser| parser.eat_expression())
            .for_node_type(NodeType::Definition)?;

        // optional default value: `= <expr>`
        let default = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression().for_node_type(NodeType::Definition)?)
        } else {
            None
        };

        let field_id = self.tree.insert(
            VariantField {
                visibility,
                name,
                ty,
                default,
            },
            self.get_span_from(start),
        );
        Ok(field_id)
    }

    /// Eat a struct body (without the header or `{` and `}`).
    pub fn eat_variant_body_mixed(
        &mut self,
        allow_fields: bool,
    ) -> ParserResult<(Vec<NodeId<VariantField>>, Vec<NodeId<Expression>>)> {
        // eat everything
        let mut fields: Vec<NodeId<VariantField>> = Vec::new();
        let mut expressions: Vec<NodeId<Expression>> = Vec::new();
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // struct field
            else if allow_fields && self.peek_variant_field().is_ok() {
                let field = self
                    .eat_variant_field()
                    .for_node_type(NodeType::VariantField)?;
                fields.push(field);
            }
            // eat expressions
            else {
                let expression_id = self
                    .try_eat_expression_as_statement()
                    .for_node_type(NodeType::Expression)?;
                expressions.push(expression_id);
            }
        }

        Ok((fields, expressions))
    }
}
