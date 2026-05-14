use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_dir::{Keyword, LocalNodeId, StringId, TokenType, TupleElement, TypeExpression};

impl Parser {
    /// Return whether the current token starts a type tuple head.
    pub(crate) fn starts_type_tuple_head(&mut self) -> bool {
        // readonly is a tuple modifier only where bracket singles stay tuples
        let is_readonly_element = !self.language.is_destack() && self.is_keyword(Keyword::Readonly);

        self.peek_is(TokenType::Spread)
            || is_readonly_element
            || self.starts_labeled_type_tuple_head()
    }

    /// Return whether the current token starts a labeled type tuple head.
    pub(crate) fn starts_labeled_type_tuple_head(&mut self) -> bool {
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
    pub(crate) fn current_type_tuple_element_is_optional(&mut self, terminator: TokenType) -> bool {
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
        let is_readonly = if !self.language.is_destack()
            && next.token.ty == TokenType::Identifier
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

    /// Recover one failed type tuple head and continue the tuple body.
    pub(crate) fn recover_type_tuple_head(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        error: ParseError,
    ) -> ParseResult<Vec<LocalNodeId<TupleElement>>> {
        // recover to the next item boundary
        self.try_recover_in_item_list(start, terminator, Some(error))?;
        let first_element = self.insert_node(TupleElement::Error, self.get_span_from(start));

        // continue after a recovered head when the tuple list has more elements
        let mut elements = vec![first_element];
        if self.peek_is(TokenType::Comma) {
            self.eat_item_stop()?;
            elements.extend(self.eat_type_tuple_elements_body(terminator)?);
        }

        Ok(elements)
    }

    /// Eat the rest of a type tuple after one unlabeled value head.
    pub(crate) fn eat_type_tuple_tail(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        value: LocalNodeId<TypeExpression>,
    ) -> ParseResult<Vec<LocalNodeId<TupleElement>>> {
        // optional marker on an unlabeled tuple element
        let is_optional = if self.current_type_tuple_element_is_optional(terminator) {
            self.bump(); // eat ?
            true
        } else {
            false
        };

        // head element
        let first_element = self.insert_node(
            TupleElement::Element {
                label: None,
                value,
                is_optional,
                is_readonly: false,
            },
            self.get_span_from(start),
        );

        // remaining elements
        let mut elements = vec![first_element];
        if self.peek_is(TokenType::Comma) {
            self.eat_item_stop()?;
            elements.extend(self.eat_type_tuple_elements_body(terminator)?);
        }

        Ok(elements)
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
