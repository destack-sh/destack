use destack_source::Span;

use crate::{Block, Function, Global, Local, LocalNodeId, Type, TypedValue, Value};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Parse a value reference.
    pub(super) fn parse_value(&mut self) -> ParseResult<Value> {
        let (value, _) = self.parse_value_with_span()?;
        Ok(value)
    }

    /// Parse a typed destination value reference.
    pub(super) fn parse_typed_destination(
        &mut self,
    ) -> ParseResult<(Value, LocalNodeId<Type>, Span)> {
        let (value, span) = self.parse_value_with_span()?;
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;
        Ok((value, ty, span))
    }

    /// Parse a value reference and return its span.
    pub(super) fn parse_value_with_span(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value", self.pos()))?;
        if token.ty != TokenType::Value {
            return Err(ParseError::unexpected("value", token.ty, token.start));
        }
        let text = token.text.to_string();
        let span = self.span_for_token(token);
        self.bump();

        let idx: u32 = text
            .strip_prefix('v')
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("value reference", span.start as usize))?;
        Ok((Value::new(idx), span))
    }

    /// Parse a block reference.
    pub(super) fn parse_block_ref(&mut self) -> ParseResult<LocalNodeId<Block>> {
        let token = self.eat_token(TokenType::BlockRefence)?;
        let idx: u32 = token
            .text
            .strip_prefix("block")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("block reference", token.start))?;
        Ok(LocalNodeId::new(idx))
    }

    /// Parse a local reference.
    pub(super) fn parse_local_ref(&mut self) -> ParseResult<LocalNodeId<Local>> {
        let token = self.eat_token(TokenType::LocalReference)?;
        let idx: u32 = token
            .text
            .strip_prefix("local")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("local reference", token.start))?;
        Ok(LocalNodeId::new(idx))
    }

    /// Parse a function reference (@name).
    pub(super) fn parse_function_reference(&mut self) -> ParseResult<LocalNodeId<Function>> {
        self.eat_token(TokenType::At)?;
        let (name, start) = self.parse_symbol_name()?;

        self.function_map
            .get(&name)
            .copied()
            .ok_or_else(|| ParseError::invalid(&format!("function reference '@{name}'"), start))
    }

    /// Parse a global reference (@name).
    pub(super) fn parse_global_reference(&mut self) -> ParseResult<LocalNodeId<Global>> {
        self.eat_token(TokenType::At)?;
        let (name, start) = self.parse_symbol_name()?;

        self.global_map
            .get(&name)
            .copied()
            .ok_or_else(|| ParseError::invalid(&format!("global reference '@{name}'"), start))
    }

    /// Parse call arguments: (v0, v1, ...).
    pub(super) fn parse_call_arguments(&mut self) -> ParseResult<Vec<Value>> {
        self.eat_token(TokenType::OpenParen)?;
        let args = self.parse_value_list()?;
        self.eat_token(TokenType::CloseParen)?;
        Ok(args)
    }

    /// Parse a comma-separated list of values.
    pub(super) fn parse_value_list(&mut self) -> ParseResult<Vec<Value>> {
        let mut values = Vec::new();
        while self.peek_token(TokenType::Value) {
            values.push(self.parse_value()?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }

    /// Parse a comma-separated list of typed values.
    pub(super) fn parse_typed_value_list(&mut self) -> ParseResult<Vec<TypedValue>> {
        let mut values = Vec::new();
        while self.peek_token(TokenType::Value) {
            let value = self.parse_value()?;
            self.eat_token(TokenType::Colon)?;
            let ty = self.parse_type()?;
            values.push(TypedValue::new(value, ty));
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }
}
