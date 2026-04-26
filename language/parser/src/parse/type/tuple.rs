use crate::{ParseError, ParseResult, Parser};

use destack_ast::{LocalNodeId, StringId, TokenType, TupleElement};

impl Parser {
    /// Return whether the current token starts a labeled type tuple head.
    fn starts_labeled_type_tuple_head(&mut self) -> bool {
        self.peek_is(TokenType::Identifier)
            && (self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Colon)
            }) || self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Maybe)
            }) && self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                parser.peek_is(TokenType::Colon)
            }))
    }

    /// Return whether one trailing `?` belongs to the surrounding type tuple element.
    fn current_type_tuple_element_is_optional(&mut self, terminator: TokenType) -> bool {
        if !self.peek_is(TokenType::Maybe) {
            return false;
        }

        let next_token_type = self.next_token_type();

        next_token_type == TokenType::Comma || next_token_type == terminator
    }

    /// Eat one labeled type tuple head.
    fn eat_labeled_type_tuple_head(&mut self) -> ParseResult<(StringId, bool)> {
        let (label, _) = self.eat_identifier_with_span()?;

        let is_optional = if self.peek_is(TokenType::Maybe) {
            self.bump(); // eat ?
            true
        } else {
            false
        };

        self.eat_token(TokenType::Colon)?;

        Ok((label, is_optional))
    }

    /// Eat one type tuple element directly in type space.
    fn eat_type_tuple_element(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<LocalNodeId<TupleElement>> {
        let start = self.span_start();

        // readonly element modifier
        let next = *self.peek()?;
        let is_readonly = if next.token.ty == TokenType::Identifier
            && self.get_span_str(next.span) == "readonly"
        {
            self.bump(); // eat readonly
            true
        } else {
            false
        };

        // spread element
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat ...

            // readonly spread elements are not modeled in the tuple IR
            if is_readonly {
                return Err(ParseError::unexpected(self.get_span_from(&start)));
            }

            // spread labels: `...rest: T`
            let label = if self.starts_labeled_type_tuple_head() {
                let (label, is_optional) = self.eat_labeled_type_tuple_head()?;
                if is_optional {
                    return Err(ParseError::unexpected(self.get_span_from(&start)));
                }

                Some(label)
            } else {
                None
            };

            // spread payload
            let value = self.eat_type_expression()?;
            let element_id = self.insert_node(
                TupleElement::Spread { label, value },
                self.get_span_from(&start),
            );

            return Ok(element_id);
        }

        // labeled head: `label: T` or `label?: T`
        let (label, is_optional) = if self.starts_labeled_type_tuple_head() {
            let (label, is_optional) = self.eat_labeled_type_tuple_head()?;
            (Some(label), is_optional)
        } else {
            (None, false)
        };

        // named rest payload: `label: ...T`
        if label.is_some() && self.peek_is(TokenType::Spread) {
            self.bump(); // eat ...

            if is_optional || is_readonly {
                return Err(ParseError::unexpected(self.get_span_from(&start)));
            }

            let value = self.eat_type_expression()?;
            let element_id = self.insert_node(
                TupleElement::Spread { label, value },
                self.get_span_from(&start),
            );

            return Ok(element_id);
        }

        // element payload
        let value = self.eat_type_expression()?;

        // trailing optional marker: `T?`
        let is_optional = if label.is_some() {
            is_optional
        } else if self.current_type_tuple_element_is_optional(terminator) {
            self.bump(); // eat ?
            true
        } else {
            false
        };

        let element_id = self.insert_node(
            TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            },
            self.get_span_from(&start),
        );

        Ok(element_id)
    }

    /// Eat type tuple elements until one closing token.
    pub(crate) fn eat_type_tuple_elements_body(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<TupleElement>>> {
        let mut element_ids = Vec::new();

        while self.has_more_tokens() {
            // closing token
            if self.peek_is(terminator) {
                break;
            }

            // one tuple element
            let element_start = self.span_start();
            let is_recovered_element;
            let element_id = match self.eat_type_tuple_element(terminator) {
                Ok(element_id) => {
                    is_recovered_element = matches!(self.tree.get(element_id), TupleElement::Error);
                    element_id
                }
                Err(error) => {
                    self.try_recover_in_item_list(&element_start, terminator, Some(error))?;
                    let element_id =
                        self.insert_node(TupleElement::Error, self.get_span_from(&element_start));
                    is_recovered_element = true;
                    element_id
                }
            };

            element_ids.push(element_id);

            // separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop()?;
            }
            // recovery boundary
            else if !self.can_continue_after_recovered_item(terminator, is_recovered_element) {
                break;
            }
        }

        Ok(element_ids)
    }
}
