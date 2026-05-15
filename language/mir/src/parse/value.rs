use crate::source::TokenType;
use destack_source::Span;

use crate::{
    BlockReference, FunctionReference, GlobalReference, LocalNodeId, LocalReference, Parameter,
    Type, TypedValueSpan, Value, ValueReference,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

impl Parser {
    /// Parse a value reference.
    pub(super) fn parse_value(&mut self) -> ParseResult<ValueReference> {
        let (value, _) = self.parse_value_reference_part()?;
        Ok(value)
    }

    /// Parse a value reference and append its span as one source segment.
    pub(super) fn parse_value_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<ValueReference> {
        let (value, span) = self.parse_value_reference_part()?;
        segment_spans.push(span);

        Ok(value)
    }

    /// Parse a typed destination value reference.
    pub(super) fn parse_typed_destination_parts(
        &mut self,
    ) -> ParseResult<(Value, LocalNodeId<Type>, Span, Span)> {
        let (value, value_span) = self.parse_value_definition_part()?;
        self.eat_token(TokenType::Colon)?;
        let (ty, type_span) = self.parse_type_part()?;
        Ok((value, ty, value_span, type_span))
    }

    /// Parse a value definition and return its span.
    pub(super) fn parse_value_definition_part(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value definition", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Value => {
                let text = self.tree.source_text(token.span).to_string();
                self.bump();

                let idx: u32 = text
                    .strip_prefix('v')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| {
                        ParseError::invalid_at_span(
                            "value definition",
                            span.start as usize,
                            span.end.saturating_sub(span.start) as usize,
                        )
                    })?;
                self.next_value_id = self.next_value_id.max(idx + 1);
                Ok((Value::new(idx), span))
            }
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
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
                self.token_type(token),
                token.start,
            )),
        }
    }

    /// Parse a value reference and return its span.
    pub(super) fn parse_value_reference_part(&mut self) -> ParseResult<(ValueReference, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value reference", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Value => {
                let text = self.tree.source_text(token.span).to_string();
                self.bump();

                let idx: u32 = text
                    .strip_prefix('v')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| {
                        ParseError::invalid_at_span(
                            "value reference",
                            span.start as usize,
                            span.end.saturating_sub(span.start) as usize,
                        )
                    })?;
                Ok((ValueReference::Value(Value::new(idx)), span))
            }
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start;
                self.bump();

                let value = self
                    .value_name_map
                    .get(&name)
                    .copied()
                    .map(ValueReference::Value)
                    .ok_or_else(|| ParseError::new(format!("undefined value '{name}'"), start))?;
                Ok((value, span))
            }
            _ => Err(ParseError::unexpected_token("value reference", token)),
        }
    }

    /// Parse a block reference.
    pub(super) fn parse_block_ref(&mut self) -> ParseResult<BlockReference> {
        let (block, _) = self.parse_block_ref_part()?;
        Ok(block)
    }

    /// Parse a block reference and return its span.
    pub(super) fn parse_block_ref_part(&mut self) -> ParseResult<(BlockReference, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("block reference", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::BlockReference => {
                let text = self.tree.source_text(token.span).to_string();
                let start = token.start;
                self.bump();

                let label_index: u32 = text
                    .strip_prefix('b')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| {
                        ParseError::invalid_at_span(
                            "block reference",
                            start,
                            span.end.saturating_sub(span.start) as usize,
                        )
                    })?;
                self.block_id_by_label_index
                    .get(&label_index)
                    .copied()
                    .map(|block| (BlockReference::Block(block), span))
                    .ok_or_else(|| ParseError::new(format!("undefined block '{text}'"), start))
            }
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start;
                self.bump();
                self.block_name_map
                    .get(&name)
                    .copied()
                    .map(|block| (BlockReference::Block(block), span))
                    .ok_or_else(|| ParseError::new(format!("undefined block '{name}'"), start))
            }
            _ => Err(ParseError::unexpected_token("block reference", token)),
        }
    }

    /// Parse a local reference and return its span.
    pub(super) fn parse_local_ref_part(&mut self) -> ParseResult<(LocalReference, Span)> {
        let token = self.eat_token(TokenType::LocalReference)?;
        let token_start = token.start;
        let token_text = self.tree.source_text(token.span).to_string();
        let token_length = token_text.len();
        let span = self.span_at(token_start, token_length);
        let idx: u32 = token_text
            .strip_prefix("local")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                ParseError::invalid_at_span("local reference", token_start, token_length)
            })?;
        Ok((LocalReference::Local(LocalNodeId::new(idx)), span))
    }

    /// Parse a local reference and append its span as one source segment.
    pub(super) fn parse_local_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<LocalReference> {
        let (local, span) = self.parse_local_ref_part()?;
        segment_spans.push(span);

        Ok(local)
    }

    /// Parse a function reference and return its span.
    pub(super) fn parse_function_reference_part(
        &mut self,
    ) -> ParseResult<(FunctionReference, Span)> {
        let (name, start) = self.parse_symbol_name()?;
        let span = self.span_at(start, name.len());

        self.function_map
            .get(&name)
            .copied()
            .map(|function| (FunctionReference::Function(function), span))
            .ok_or_else(|| ParseError::invalid(&format!("function reference '{name}'"), start))
    }

    /// Parse a function reference and append its span as one source segment.
    pub(super) fn parse_function_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<FunctionReference> {
        let (function, span) = self.parse_function_reference_part()?;
        segment_spans.push(span);

        Ok(function)
    }

    /// Parse a global reference and return its span.
    pub(super) fn parse_global_reference_part(&mut self) -> ParseResult<(GlobalReference, Span)> {
        let (name, start) = self.parse_symbol_name()?;
        let span = self.span_at(start, name.len());

        self.global_map
            .get(&name)
            .copied()
            .map(|global| (GlobalReference::Global(global), span))
            .ok_or_else(|| ParseError::invalid(&format!("global reference '{name}'"), start))
    }

    /// Parse a global reference and append its span as one source segment.
    pub(super) fn parse_global_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<GlobalReference> {
        let (global, span) = self.parse_global_reference_part()?;
        segment_spans.push(span);

        Ok(global)
    }

    /// Parse call arguments and append each argument span as one source segment.
    pub(super) fn parse_call_argument_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<Vec<ValueReference>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let args = self.parse_value_list_segments(segment_spans)?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(args)
    }

    /// Parse a comma-separated list of values.
    pub(super) fn parse_value_list(&mut self) -> ParseResult<Vec<ValueReference>> {
        let mut values = Vec::new();
        while self.is_value_reference_start() {
            values.push(self.parse_value()?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }

    /// Parse a comma separated list of values and append their spans as source segments.
    pub(super) fn parse_value_list_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<Vec<ValueReference>> {
        let mut values = Vec::new();
        while self.is_value_reference_start() {
            values.push(self.parse_value_segment(segment_spans)?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        Ok(values)
    }

    /// Parse a comma-separated list of typed values and their spans.
    pub(super) fn parse_typed_values(
        &mut self,
    ) -> ParseResult<(Vec<Parameter>, Vec<TypedValueSpan>)> {
        let mut values = Vec::new();
        let mut spans = Vec::new();
        while self.is_value_definition_start() {
            let value_start = self.pos();
            let (value, name_span) = self.parse_value_definition_part()?;
            let colon_token = self.eat_token(TokenType::Colon)?;
            let (ty, type_span) = self.parse_type_reference_after(colon_token, "parameter type");
            let value_span = self.span_from_parse_start(value_start);
            values.push(Parameter {
                value: ValueReference::Value(value),
                ty,
            });
            spans.push(TypedValueSpan::new(value_span, Some(name_span), type_span));
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        Ok((values, spans))
    }
}
