use destack_source::Span;

use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, AttributeKeyValue, AttributeValue, FloatValue,
    IntegerReference, TypeReference,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

#[allow(clippy::type_complexity)]
impl Parser {
    /// Parse an attribute list prefix and retain each attribute span.
    pub(super) fn parse_attributes(&mut self) -> ParseResult<(Vec<Attribute>, Vec<Span>)> {
        // attribute blocks
        let mut attributes = Vec::new();
        let mut spans = Vec::new();

        while self.peek_token(TokenType::At) {
            let attribute_start = self.pos();
            let attribute = self.parse_attribute()?;
            let attribute_span = self.span_from_parse_start(attribute_start);
            attributes.push(attribute);
            spans.push(attribute_span);
        }

        Ok((attributes, spans))
    }

    /// Parse a single attribute.
    fn parse_attribute(&mut self) -> ParseResult<Attribute> {
        // attribute header
        self.eat_token(TokenType::At)?;

        // name and arguments
        let name_token = if self.peek_token(TokenType::Ownership) {
            self.eat_token(TokenType::Ownership)?
        } else {
            self.eat_token(TokenType::Identifier)?
        };
        let name_text = self.tree.source_text(name_token.span).to_string();
        let name = AttributeIdentifier::Identifier(self.strings.intern(&name_text));

        // optional argument payload
        let args = if self.eat_token_maybe(TokenType::OpenParen) {
            let args = self.parse_attribute_args()?;
            self.eat_token(TokenType::CloseParen)?;
            args
        } else {
            AttributeArgs::None
        };

        Ok(Attribute { name, args })
    }

    /// Parse a single attribute argument list.
    fn parse_attribute_args(&mut self) -> ParseResult<AttributeArgs> {
        // empty list
        if self.peek_token(TokenType::CloseParen) {
            return Ok(AttributeArgs::None);
        }

        // key value list
        if self.peek_token(TokenType::Identifier)
            && self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::Equals)
        {
            let mut pairs = Vec::new();
            loop {
                let key_token = self.eat_token(TokenType::Identifier)?;
                let key_text = self.tree.source_text(key_token.span).to_string();
                let key = AttributeIdentifier::Identifier(self.strings.intern(&key_text));
                self.eat_token(TokenType::Equals)?;
                let value = self.parse_attribute_value()?;
                pairs.push(AttributeKeyValue { key, value });

                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            return Ok(AttributeArgs::KeyValues(pairs));
        }

        // single value or list
        let first = self.parse_attribute_value()?;
        if !self.eat_token_maybe(TokenType::Comma) {
            return Ok(AttributeArgs::Value(first));
        }

        // value list
        let mut values = vec![first];
        loop {
            let value = self.parse_attribute_value()?;
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        Ok(AttributeArgs::Values(values))
    }

    /// Parse a single attribute value.
    fn parse_attribute_value(&mut self) -> ParseResult<AttributeValue> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("attribute value", self.pos()))?;
        let token_ty = token.ty;
        let token_text = self.tree.source_text(token.span).to_string();
        let token_start = token.start;

        // type values
        if self.peek_type(token_ty) {
            let ty = self.parse_type()?;
            return Ok(AttributeValue::Type(TypeReference::Type(ty)));
        }

        // scalar and list values
        match token_ty {
            TokenType::Identifier => {
                self.bump();
                let identifier = AttributeIdentifier::Identifier(self.strings.intern(&token_text));

                Ok(AttributeValue::Identifier(identifier))
            }
            TokenType::BooleanLiteral => {
                self.bump();
                let value = token_text == "true";
                Ok(AttributeValue::Boolean(value))
            }
            TokenType::IntLiteral => {
                let value = self.parse_int_literal()?;
                Ok(AttributeValue::Integer(IntegerReference::Integer(value)))
            }
            TokenType::FloatLiteral => {
                self.bump();
                let value = self.parse_attribute_float(&token_text, token_start)?;
                Ok(AttributeValue::Float(value))
            }
            TokenType::StringLiteral => {
                self.bump();
                let value = self.parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(AttributeValue::String(self.strings.intern(&value)))
            }
            TokenType::OpenBracket => {
                // list literal
                self.eat_token(TokenType::OpenBracket)?;
                let mut values = Vec::new();
                while !self.peek_token(TokenType::CloseBracket) {
                    values.push(self.parse_attribute_value()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBracket)?;

                Ok(AttributeValue::List(values))
            }
            _ => Err(ParseError::unexpected(
                "attribute value",
                token_ty,
                token_start,
            )),
        }
    }

    /// Parse a float literal for attributes.
    fn parse_attribute_float(&mut self, text: &str, start: usize) -> ParseResult<FloatValue> {
        // split off a type suffix when present
        let (digits, width) = if let Some(pos) = text.rfind('f') {
            let (digits, suffix) = text.split_at(pos);
            let suffix_digits = &suffix[1..];
            if !suffix_digits.is_empty() && suffix_digits.chars().all(|c| c.is_ascii_digit()) {
                let width = suffix_digits
                    .parse::<u16>()
                    .map_err(|_| ParseError::invalid("float width", start))?;
                (digits, Some(width))
            } else {
                (text, None)
            }
        } else {
            (text, None)
        };

        // normalize numeric separators
        let digits = digits.replace('_', "");

        // parse the numeric value
        let value = match width {
            Some(32) => {
                let value: f32 = digits
                    .parse()
                    .map_err(|_| ParseError::invalid("float literal", start))?;
                value as f64
            }
            Some(64) => digits
                .parse::<f64>()
                .map_err(|_| ParseError::invalid("float literal", start))?,
            Some(_) => {
                return Err(ParseError::invalid("float width", start));
            }
            None => digits
                .parse::<f64>()
                .map_err(|_| ParseError::invalid("float literal", start))?,
        };

        Ok(FloatValue::from_f64(value))
    }
}
