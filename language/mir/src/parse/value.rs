use crate::source::TokenType;
use destack_source::Span;

use crate::{
    BlockId, BlockParameter, FunctionId, GenericArgument, GlobalId, LocalId, LocalNodeId, Place,
    Projection, Type, TypedValueSpan, Value,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

impl Parser {
    /// Parse a storage root and its projections.
    pub(super) fn parse_place(&mut self, segments: &mut Vec<Span>) -> ParseResult<Place> {
        // read a root or a parenthesized dereference or case selection
        let mut place = if self.eat_token_if(TokenType::OpenParenthesis) {
            let is_dereference = self.eat_token_if(TokenType::Star);
            let mut place = self.parse_place(segments)?;
            if is_dereference {
                place.push(Projection::Deref);
            }
            if self
                .peek()
                .is_some_and(|token| self.tree.source_text(token.span) == "as")
            {
                self.eat_token(TokenType::Identifier)?;
                let case = self.parse_place_index(segments)?;
                place.push(Projection::Variant { case });
            }
            self.eat_token(TokenType::CloseParenthesis)?;

            place
        } else if self.eat_token_if(TokenType::At) {
            Place::global(self.parse_global_segment(segments)?)
        } else if self.peek().is_some_and(|token| {
            self.local_name_map
                .contains_key(self.tree.source_text(token.span))
        }) {
            Place::local(self.parse_local_segment(segments)?)
        } else {
            Place::value(self.parse_value_segment(segments)?)
        };

        // append fields and indexed projections
        loop {
            if self.eat_token_if(TokenType::Dot) {
                let index = self.parse_place_index(segments)?;
                place.push(Projection::Field { index });
            } else if self.eat_token_if(TokenType::OpenBracket) {
                let projection = if self.peek_is(TokenType::Integer) {
                    Projection::Element {
                        index: self.parse_place_index(segments)?,
                    }
                } else {
                    let index = self.parse_value_segment(segments)?;
                    if self.eat_token_if(TokenType::Semicolon) {
                        let length = self.parse_value_segment(segments)?;

                        Projection::Slice {
                            start: index,
                            length,
                        }
                    } else {
                        Projection::Index { index }
                    }
                };
                self.eat_token(TokenType::CloseBracket)?;
                place.push(projection);
            } else {
                break;
            }
        }

        Ok(place)
    }

    /// Parse a concrete field, element, or case index.
    fn parse_place_index(&mut self, segments: &mut Vec<Span>) -> ParseResult<u32> {
        let (index, span) = self.parse_int_literal_part()?;
        segments.push(span);

        u32::try_from(index).map_err(|_| ParseError::invalid("place index", span.start as usize))
    }

    /// Parse a value reference.
    pub(super) fn parse_value(&mut self) -> ParseResult<Value> {
        let (value, _) = self.parse_value_reference_part()?;
        Ok(value)
    }

    /// Parse a value reference and append its span as one source segment.
    pub(super) fn parse_value_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<Value> {
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
        let (ty, type_span) = self.parse_type_use_part()?;
        Ok((value, ty, value_span, type_span))
    }

    /// Parse a value definition and return its span.
    pub(super) fn parse_value_definition_part(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value definition", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start();
                self.bump();

                let value = self.parse_value_id(&name, start)?;
                if !self.defined_values.insert(value) {
                    return Err(ParseError::new(format!("duplicate value '{name}'"), start));
                }

                self.next_value_id = self.next_value_id.max(value.id() + 1);

                Ok((value, span))
            }
            _ => Err(ParseError::unexpected(
                "value definition",
                self.token_type(token),
                token.start(),
            )),
        }
    }

    /// Parse one canonical SSA value identity.
    pub(super) fn parse_value_id(&self, name: &str, start: usize) -> ParseResult<Value> {
        let id = name
            .strip_prefix('v')
            .and_then(|id| id.parse::<u32>().ok())
            .ok_or_else(|| ParseError::new(format!("invalid value '{name}'"), start))?;

        Ok(Value::new(id))
    }

    /// Parse a value reference and return its span.
    pub(super) fn parse_value_reference_part(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value reference", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start();
                self.bump();

                let value = self.parse_value_id(&name, start)?;
                if !self.defined_values.contains(&value) {
                    return Err(ParseError::new(format!("undefined value '{name}'"), start));
                }

                Ok((value, span))
            }
            _ => Err(ParseError::unexpected_token("value reference", token)),
        }
    }

    /// Parse a block reference.
    pub(super) fn parse_block_ref(&mut self) -> ParseResult<BlockId> {
        let (block, _) = self.parse_block_ref_part()?;
        Ok(block)
    }

    /// Parse a block reference and return its span.
    pub(super) fn parse_block_ref_part(&mut self) -> ParseResult<(BlockId, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("block reference", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start();
                self.bump();

                self.block_name_map
                    .get(&name)
                    .copied()
                    .map(|block| (block, span))
                    .ok_or_else(|| ParseError::new(format!("undefined block '{name}'"), start))
            }
            _ => Err(ParseError::unexpected_token("block reference", token)),
        }
    }

    /// Parse a local reference and return its span.
    pub(super) fn parse_local_ref_part(&mut self) -> ParseResult<(LocalId, Span)> {
        let token = self.eat_token(TokenType::Identifier)?;
        let token_start = token.start();
        let token_text = self.tree.source_text(token.span).to_string();
        let token_length = token_text.len();
        let span = self.span_at(token_start, token_length);
        let local = self
            .local_name_map
            .get(&token_text)
            .copied()
            .ok_or_else(|| {
                ParseError::invalid_with_length("local reference", token_start, token_length)
            })?;

        Ok((local, span))
    }

    /// Parse a local reference and append its span as one source segment.
    pub(super) fn parse_local_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<LocalId> {
        let (local, span) = self.parse_local_ref_part()?;
        segment_spans.push(span);

        Ok(local)
    }

    /// Parse a function reference and return its generic arguments and span.
    pub(super) fn parse_function_reference_part(
        &mut self,
    ) -> ParseResult<(FunctionId, Vec<GenericArgument>, Span)> {
        let (name, start) = self.parse_symbol_name()?;
        let arguments = self.parse_function_arguments()?;
        let span = self.span_between(start, self.pos());

        // a declared specialization
        if let Some(function) = self.function_map.get(&(name.clone(), arguments.clone())) {
            return Ok((*function, Vec::new(), span));
        }

        // a template applied to the arguments
        if let Some(template) = self.function_map.get(&(name.clone(), Vec::new()))
            && !arguments.is_empty()
        {
            return Ok((*template, arguments, span));
        }

        Err(ParseError::invalid(
            &format!("function reference '{name}'"),
            start,
        ))
    }

    /// Parse a global reference and return its span.
    pub(super) fn parse_global_reference_part(&mut self) -> ParseResult<(GlobalId, Span)> {
        let (name, start) = self.parse_symbol_name()?;
        let span = self.span_at(start, name.len());

        self.global_map
            .get(&name)
            .copied()
            .map(|global| (global, span))
            .ok_or_else(|| ParseError::invalid(&format!("global reference '{name}'"), start))
    }

    /// Parse a global reference and append its span as one source segment.
    pub(super) fn parse_global_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<GlobalId> {
        let (global, span) = self.parse_global_reference_part()?;
        segment_spans.push(span);

        Ok(global)
    }

    /// Parse call arguments and append each argument span as one source segment.
    pub(super) fn parse_call_argument_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<Vec<Value>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let args = self.parse_value_list_segments(segment_spans)?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(args)
    }

    /// Parse a comma-separated list of values.
    pub(super) fn parse_value_list(&mut self) -> ParseResult<Vec<Value>> {
        let mut values = Vec::new();
        while self.is_value_reference_start() {
            values.push(self.parse_value()?);
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }

    /// Parse a comma separated list of values and append their spans as source segments.
    pub(super) fn parse_value_list_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<Vec<Value>> {
        let mut values = Vec::new();
        while self.is_value_reference_start() {
            values.push(self.parse_value_segment(segment_spans)?);
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        Ok(values)
    }

    /// Parse a comma-separated list of typed values and their spans.
    pub(super) fn parse_typed_values(
        &mut self,
    ) -> ParseResult<(Vec<BlockParameter>, Vec<TypedValueSpan>)> {
        let mut values = Vec::new();
        let mut spans = Vec::new();
        while self.is_value_definition_start() {
            let value_start = self.pos();
            let (value, name_span) = self.parse_value_definition_part()?;
            let colon_token = self.eat_token(TokenType::Colon)?;
            let (ty, type_span) = self.parse_type_use_after(colon_token, "parameter type");
            let value_span = self.span_from_parse_start(value_start);
            values.push(BlockParameter { value, ty });
            spans.push(TypedValueSpan::new(value_span, Some(name_span), type_span));
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        Ok((values, spans))
    }
}
