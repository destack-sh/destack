use destack_source::Span;

use crate::{Block, Function, Global, Local, LocalNodeId, Type, TypedValue, Value};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Parse a value reference.
    pub(super) fn parse_value(&mut self) -> ParseResult<Value> {
        let (value, _) = self.parse_value_reference_with_span()?;
        Ok(value)
    }

    /// Parse a typed destination value reference.
    pub(super) fn parse_typed_destination(
        &mut self,
    ) -> ParseResult<(Value, LocalNodeId<Type>, Span)> {
        let (value, span) = self.parse_value_definition_with_span()?;
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;
        Ok((value, ty, span))
    }

    /// Parse a value definition and return its span.
    pub(super) fn parse_value_definition_with_span(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value definition", self.pos()))?;
        let span = self.span_for_token(token);

        match token.ty {
            TokenType::Value => {
                let text = token.text.to_string();
                self.bump();

                let idx: u32 = text
                    .strip_prefix('v')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| ParseError::invalid("value definition", span.start as usize))?;
                self.next_value_id = self.next_value_id.max(idx + 1);
                Ok((Value::new(idx), span))
            }
            TokenType::Identifier => {
                let name = token.text.to_string();
                let start = token.start;
                self.bump();

                if self.value_name_map.contains_key(&name) {
                    return Err(ParseError::new(
                        format!("duplicate value name '{name}'"),
                        start,
                    ));
                }

                let value = Value::new(self.next_value_id);
                self.next_value_id += 1;
                self.value_name_map.insert(name.clone(), value);

                if let Some(function_id) = self.current_function {
                    let name_id = self.strings.intern(&name);
                    let function = self.tree.get_mut(function_id);
                    let index = value.0 as usize;
                    if index >= function.value_names.len() {
                        function.value_names.resize(index + 1, None);
                    }
                    function.value_names[index] = Some(name_id);
                }

                Ok((value, span))
            }
            _ => Err(ParseError::unexpected(
                "value definition",
                token.ty,
                token.start,
            )),
        }
    }

    /// Parse a value reference and return its span.
    pub(super) fn parse_value_reference_with_span(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value reference", self.pos()))?;
        let span = self.span_for_token(token);

        match token.ty {
            TokenType::Value => {
                let text = token.text.to_string();
                self.bump();

                let idx: u32 = text
                    .strip_prefix('v')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| ParseError::invalid("value reference", span.start as usize))?;
                Ok((Value::new(idx), span))
            }
            TokenType::Identifier => {
                let name = token.text.to_string();
                let start = token.start;
                self.bump();

                let value =
                    self.value_name_map.get(&name).copied().ok_or_else(|| {
                        ParseError::new(format!("undefined value '{name}'"), start)
                    })?;
                Ok((value, span))
            }
            _ => Err(ParseError::unexpected(
                "value reference",
                token.ty,
                token.start,
            )),
        }
    }

    /// Parse a block reference.
    pub(super) fn parse_block_ref(&mut self) -> ParseResult<LocalNodeId<Block>> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("block reference", self.pos()))?;

        match token.ty {
            TokenType::BlockRefence => {
                let text = token.text.to_string();
                let start = token.start;
                self.bump();

                let label_index: u32 = text
                    .strip_prefix('b')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| ParseError::invalid("block reference", start))?;
                self.block_id_by_label_index
                    .get(&label_index)
                    .copied()
                    .ok_or_else(|| ParseError::new(format!("undefined block '{text}'"), start))
            }
            TokenType::Identifier => {
                let name = token.text.to_string();
                let start = token.start;
                self.bump();
                self.block_name_map
                    .get(&name)
                    .copied()
                    .ok_or_else(|| ParseError::new(format!("undefined block '{name}'"), start))
            }
            _ => Err(ParseError::unexpected(
                "block reference",
                token.ty,
                token.start,
            )),
        }
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

    /// Parse a function reference.
    pub(super) fn parse_function_reference(&mut self) -> ParseResult<LocalNodeId<Function>> {
        let (name, start) = self.parse_symbol_name()?;

        self.function_map
            .get(&name)
            .copied()
            .ok_or_else(|| ParseError::invalid(&format!("function reference '{name}'"), start))
    }

    /// Parse a global reference.
    pub(super) fn parse_global_reference(&mut self) -> ParseResult<LocalNodeId<Global>> {
        let (name, start) = self.parse_symbol_name()?;

        self.global_map
            .get(&name)
            .copied()
            .ok_or_else(|| ParseError::invalid(&format!("global reference '{name}'"), start))
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
        while self.is_value_reference_start() {
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
        while self.is_value_definition_start() {
            let (value, _) = self.parse_value_definition_with_span()?;
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
