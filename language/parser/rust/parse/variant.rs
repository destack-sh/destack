#![allow(clippy::type_complexity)]

use dyst_ast::{Expression, Keyword, Mutability, ScopedMutability};
use std::str::FromStr;

use crate::TokenType;
use crate::parse::prelude::*;

use crate::{NodeId, NodeType, Parser, ParserError, ParserResult, VariantField};

pub(crate) static VARIANT_FIELD_MODIFIERS: [Keyword; 4] = [
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
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
    /// May be preceded by any number of field modifiers.
    pub(crate) fn peek_variant_field(&self) -> ParserResult<()> {
        let mut pos = self.pos() as usize;
        // skip modifiers
        while let Some(token) = self.tokens.get(pos) {
            let keyword = Keyword::from_str(self.get_span_str(token.span));
            if let Ok(keyword) = keyword
                && VARIANT_FIELD_MODIFIERS.contains(&keyword)
            {
                pos += 1;
            } else {
                break;
            }
        }
        // we're looking for something that looks like a variant field
        if pos + 3 < self.tokens.len() {
            let token_ty = self.tokens[pos].token.ty;
            let next_token_ty = self.tokens[pos + 1].token.ty;
            let next_next_token_ty = self.tokens[pos + 2].token.ty;
            match (token_ty, next_token_ty, next_next_token_ty) {
                (TokenType::Identifier, TokenType::Colon, _)
                | (TokenType::Identifier, TokenType::Maybe, TokenType::Colon) => {
                    return Ok(());
                }
                _ => {}
            }
        }
        Err(ParserError::expected(
            self.tokens[pos].span,
            TokenType::Identifier,
        ))
    }

    /// Eat a single variant field: `T`,`name: T`, or `name: T = <expr>` (including visibility).
    ///
    /// Examples:
    /// ```
    /// T
    /// name: T
    /// name: T = <expr>
    /// public T
    /// readonly name: T
    /// baz?: T // shorthand for baz: T?
    /// ```
    pub(crate) fn eat_variant_field(&mut self) -> ParserResult<NodeId<VariantField>> {
        let start = self.mark();

        // visibility
        let visibility = self.eat_visibility_maybe()?;

        // mutability
        let mutability = if self.peek_keyword(Keyword::Readonly).is_ok() {
            self.bump(); // eat readonly
            Some(ScopedMutability::Unscoped {
                mutability: Mutability::Immutable,
            })
        } else {
            None
        };

        // name:
        let (name, is_maybe) =
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let name = self.eat_identifier()?;
                self.bump(); // eat colon
                (Some(name), false)
            } else if self.peek_identifier().is_ok()
                && self.peek_next_token(TokenType::Maybe).is_ok()
                && self.peek_next_next_token(TokenType::Colon).is_ok()
            {
                let name = self.eat_identifier()?;
                self.bump(); // eat maybe
                self.bump(); // eat colon
                (Some(name), true)
            } else {
                (None, false)
            };

        // type
        let ty = self
            .with_options(self.options.in_type(), |parser| parser.eat_expression())
            .for_node_type(NodeType::Definition)?;
        let ty = if is_maybe {
            self.tree
                .insert(Expression::Maybe(ty), self.tree.spans.get(ty))
        } else {
            ty
        };

        // optional default value: `= <expr>`
        let default = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression().for_node_type(NodeType::Definition)?)
        } else {
            None
        };

        let field_id = self.tree.insert(
            VariantField {
                mutability,
                visibility,
                name,
                ty,
                default,
            },
            self.get_span_from(start),
        );
        Ok(field_id)
    }

    /// Eat a variant body (without the header or `{` and `}`).
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
