use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

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
            && (self.token_type_at_offset(1) == TokenType::Colon
                || self.token_type_at_offset(1) == TokenType::Maybe
                    && self.token_type_at_offset(2) == TokenType::Colon)
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
    ///
    /// Examples:
    /// ```ds
    /// name: string
    /// name?: string
    /// rest: ...string[]
    /// ```
    fn eat_labeled_type_tuple_head(&mut self) -> ParserResult<(StringId, bool)> {
        let (label, _) = self.eat_identifier_with_span()?;

        let is_optional = if self.peek_is(TokenType::Maybe) {
            self.bump();
            true
        } else {
            false
        };

        self.eat_token(TokenType::Colon)?;

        Ok((label, is_optional))
    }

    /// Eat one type tuple element directly in type space.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// name?: string
    /// ...rest: string[]
    /// ```
    fn eat_type_tuple_element(
        &mut self,
        terminator: TokenType,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        let start = self.span_start();
        let is_readonly = self.eat_type_tuple_readonly_modifier()?;

        // spread element
        if self.peek_is(TokenType::Spread) {
            return self.eat_type_tuple_spread_element(&start, terminator, is_readonly);
        }

        let (label, is_optional) = self.eat_type_tuple_label_if_present()?;

        // named rest payload
        if label.is_some() && self.peek_is(TokenType::Spread) {
            return self.eat_type_tuple_named_rest_element(&start, label, is_optional, is_readonly);
        }

        self.eat_type_tuple_regular_element(&start, terminator, label, is_optional, is_readonly)
    }

    /// Eat a readonly tuple element modifier.
    ///
    /// Examples:
    /// ```ds
    /// readonly string
    /// readonly name: string
    /// readonly [string, number]
    /// ```
    fn eat_type_tuple_readonly_modifier(&mut self) -> ParserResult<bool> {
        let next = self.peek()?;
        if !self.language.is_destack()
            && next.token.ty() == TokenType::Identifier
            && self.get_span_str(next.span) == "readonly"
        {
            self.bump();
            return Ok(true);
        }

        Ok(false)
    }

    /// Eat a spread tuple element.
    ///
    /// Examples:
    /// ```ds
    /// ...string[]
    /// ...rest: string[]
    /// ...readonly string[]
    /// ```
    fn eat_type_tuple_spread_element(
        &mut self,
        start: &ParserSpanStart,
        _terminator: TokenType,
        is_readonly: bool,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        self.bump();

        if is_readonly {
            return Err(ParserError::unexpected(self.get_span_from(start)));
        }

        let label = if self.starts_labeled_type_tuple_head() {
            let (label, is_optional) = self.eat_labeled_type_tuple_head()?;
            if is_optional {
                return Err(ParserError::unexpected(self.get_span_from(start)));
            }

            Some(label)
        } else {
            None
        };

        let value = self.eat_type_expression()?;

        Ok(self.insert_node(
            TupleElement::Spread { label, value },
            self.get_span_from(start),
        ))
    }

    /// Eat a tuple element label when present.
    ///
    /// Examples:
    /// ```ds
    /// name: string
    /// name?: string
    /// ...rest: string[]
    /// ```
    fn eat_type_tuple_label_if_present(&mut self) -> ParserResult<(Option<StringId>, bool)> {
        if !self.starts_labeled_type_tuple_head() {
            return Ok((None, false));
        }

        let (label, is_optional) = self.eat_labeled_type_tuple_head()?;

        Ok((Some(label), is_optional))
    }

    /// Eat a named rest tuple element.
    ///
    /// Examples:
    /// ```ds
    /// rest: ...string[]
    /// args: ...unknown[]
    /// values: ...readonly string[]
    /// ```
    fn eat_type_tuple_named_rest_element(
        &mut self,
        start: &ParserSpanStart,
        label: Option<StringId>,
        is_optional: bool,
        is_readonly: bool,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        self.bump();

        if is_optional || is_readonly {
            return Err(ParserError::unexpected(self.get_span_from(start)));
        }

        let value = self.eat_type_expression()?;

        Ok(self.insert_node(
            TupleElement::Spread { label, value },
            self.get_span_from(start),
        ))
    }

    /// Eat a regular tuple element.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// string?
    /// name?: string
    /// ```
    fn eat_type_tuple_regular_element(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        label: Option<StringId>,
        is_optional: bool,
        is_readonly: bool,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        let value = self.eat_type_expression()?;

        let is_optional = if label.is_some() {
            is_optional
        } else if self.current_type_tuple_element_is_optional(terminator) {
            self.bump();
            true
        } else {
            false
        };

        Ok(self.insert_node(
            TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat the rest of a type tuple after one unlabeled value head.
    ///
    /// Examples:
    /// ```ds
    /// string,
    /// string, number
    /// string?, ...boolean[]
    /// ```
    pub(crate) fn eat_type_tuple_tail(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        value: LocalNodeId<TypeExpression>,
    ) -> ParserResult<Vec<LocalNodeId<TupleElement>>> {
        // optional marker on an unlabeled tuple element
        let is_optional = if self.current_type_tuple_element_is_optional(terminator) {
            self.bump();
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
    ///
    /// Examples:
    /// ```ds
    /// string, number
    /// name: string, age?: number
    /// ...rest: string[]
    /// ```
    pub(crate) fn eat_type_tuple_elements_body(
        &mut self,
        terminator: TokenType,
    ) -> ParserResult<Vec<LocalNodeId<TupleElement>>> {
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
