use crate::parse::context::TypeContext;
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{LocalNodeId, StringId, TokenType, TupleElement, TypeExpression};

/// One labeled tuple element head.
#[derive(Clone, Copy)]
struct TupleLabel {
    /// The element label.
    name: StringId,
    /// Whether the label carries an optional marker.
    is_optional: bool,
}

impl Parser {
    /// Return whether the current token starts a type tuple head.
    pub(crate) fn peek_type_tuple(&self) -> bool {
        self.peek_is(TokenType::Spread) || self.peek_labeled_tuple()
    }

    /// Return whether the current token starts a labeled type tuple head.
    pub(crate) fn peek_labeled_tuple(&self) -> bool {
        self.peek_is(TokenType::Identifier)
            && (self.peek_token_type_at(1) == TokenType::Colon
                || self.peek_token_type_at(1) == TokenType::Maybe
                    && self.peek_token_type_at(2) == TokenType::Colon)
    }

    /// Return whether one trailing `?` belongs to the surrounding type tuple element.
    pub(crate) fn peek_tuple_element_optional(&self) -> bool {
        if !self.peek_is(TokenType::Maybe) {
            return false;
        }

        let peek_next_token_type = self.peek_next_token_type();

        peek_next_token_type == TokenType::Comma
            || peek_next_token_type == TokenType::CloseParenthesis
    }

    /// Parse one labeled type tuple head.
    ///
    /// Examples:
    /// ```ds
    /// name: string
    /// name?: string
    /// rest: ...string[]
    /// ```
    fn parse_tuple_label(&mut self) -> ParserResult<TupleLabel> {
        let (name, _) = self.eat_identifier_with_range()?;

        let is_optional = if self.peek_is(TokenType::Maybe) {
            self.bump();
            true
        } else {
            false
        };

        self.eat_token(TokenType::Colon)?;

        Ok(TupleLabel { name, is_optional })
    }

    /// Parse one type tuple element directly in type space.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// name?: string
    /// ...rest: string[]
    /// ```
    fn parse_type_tuple_element(
        &mut self,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        let start = self.mark_parse_start();

        // spread element
        if self.peek_is(TokenType::Spread) {
            self.bump();
            let label = if self.peek_labeled_tuple() {
                Some(self.parse_tuple_label()?)
            } else {
                None
            };

            return self.parse_tuple_rest_element(&start, label, context);
        }

        let label = self.parse_tuple_label_if_present()?;

        // named rest payload
        if label.is_some() && self.peek_is(TokenType::Spread) {
            self.bump();

            return self.parse_tuple_rest_element(&start, label, context);
        }

        self.parse_tuple_element(&start, label, context)
    }

    /// Parse a tuple element label when present.
    ///
    /// Examples:
    /// ```ds
    /// name: string
    /// name?: string
    /// ...rest: string[]
    /// ```
    fn parse_tuple_label_if_present(&mut self) -> ParserResult<Option<TupleLabel>> {
        if !self.peek_labeled_tuple() {
            return Ok(None);
        }

        self.parse_tuple_label().map(Some)
    }

    /// Parse one tuple rest element after its spread token.
    fn parse_tuple_rest_element(
        &mut self,
        start: &ParseStart,
        label: Option<TupleLabel>,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        if label.is_some_and(|label| label.is_optional) {
            return Err(ParserError::unexpected(self.range_since(start)));
        }

        let value = self.parse_type(context.nested())?;

        Ok(self.insert_node(
            TupleElement::Spread {
                label: label.map(|label| label.name),
                value,
            },
            self.range_since(start),
        ))
    }

    /// Parse a regular tuple element.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// string?
    /// name?: string
    /// ```
    fn parse_tuple_element(
        &mut self,
        start: &ParseStart,
        label: Option<TupleLabel>,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        let value = self.parse_type(context.nested())?;

        let is_optional = if let Some(label) = label {
            label.is_optional
        } else if self.peek_tuple_element_optional() {
            self.bump();
            true
        } else {
            false
        };

        Ok(self.insert_node(
            TupleElement::Element {
                label: label.map(|label| label.name),
                value,
                is_optional,
                is_readonly: false,
            },
            self.range_since(start),
        ))
    }

    /// Parse the rest of a type tuple after one unlabeled value head.
    ///
    /// Examples:
    /// ```ds
    /// string,
    /// string, number
    /// string?, ...boolean[]
    /// ```
    pub(crate) fn parse_type_tuple_tail(
        &mut self,
        start: &ParseStart,
        value: LocalNodeId<TypeExpression>,
        context: TypeContext,
    ) -> ParserResult<Vec<LocalNodeId<TupleElement>>> {
        // optional marker on an unlabeled tuple element
        let is_optional = if self.peek_tuple_element_optional() {
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
            self.range_since(start),
        );

        // remaining elements
        let mut elements = vec![first_element];
        if self.peek_is(TokenType::Comma) {
            self.eat_token(TokenType::Comma)?;
            elements.extend(self.parse_type_tuple_elements_body(context)?);
        }

        Ok(elements)
    }

    /// Parse type tuple elements until one closing token.
    ///
    /// Examples:
    /// ```ds
    /// string, number
    /// name: string, age?: number
    /// ...rest: string[]
    /// ```
    pub(crate) fn parse_type_tuple_elements_body(
        &mut self,
        context: TypeContext,
    ) -> ParserResult<Vec<LocalNodeId<TupleElement>>> {
        let mut element_ids = Vec::new();

        while self.has_more_tokens() {
            // closing token
            if self.peek_is(TokenType::CloseParenthesis) {
                break;
            }

            // one tuple element
            let element_start = self.mark_parse_start();
            let is_recovered_element;
            let element_id = match self.parse_type_tuple_element(context) {
                Ok(element_id) => {
                    is_recovered_element = matches!(self.tree.get(element_id), TupleElement::Error);
                    element_id
                }
                Err(error) => {
                    self.recover_list_item(
                        self.range_since(&element_start),
                        TokenType::CloseParenthesis,
                        error,
                    );
                    let element_id =
                        self.insert_node(TupleElement::Error, self.range_since(&element_start));
                    is_recovered_element = true;
                    element_id
                }
            };

            element_ids.push(element_id);

            // separator
            if self.peek_is(TokenType::Comma) {
                self.eat_token(TokenType::Comma)?;
            }
            // recovery boundary
            else if !is_recovered_element
                || !self.peek_recovered_list_continuation(TokenType::CloseParenthesis)
            {
                break;
            }
        }

        Ok(element_ids)
    }
}
