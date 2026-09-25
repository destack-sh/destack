use crate::parse::{TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use tspp_dir::{LocalNodeId, StringId, TokenType, TupleElement, TypeExpression};
use tspp_source::ByteRange;

/// One labeled tuple element head.
#[derive(Clone, Copy)]
struct TupleLabel {
    /// The element label.
    name: StringId,
    /// The label source range.
    range: ByteRange,
    /// Whether the label carries an optional marker.
    is_optional: bool,
}

impl Parser {
    /// Return whether the current token starts a type tuple head.
    pub(crate) fn peek_type_tuple(&self) -> bool {
        self.peek_is(TokenType::Spread)
            || self.peek_labeled_tuple()
            || self.peek_repeated_pattern_marker()
    }

    /// Return whether the current token starts a labeled type tuple head.
    pub(crate) fn peek_labeled_tuple(&self) -> bool {
        let after_name = self.peek_token_type_at(1);

        self.peek_is(TokenType::Identifier)
            && (after_name == TokenType::Colon
                || after_name == TokenType::Maybe && self.peek_token_type_at(2) == TokenType::Colon)
    }

    /// Return whether one trailing `?` belongs to a tuple before one closing token.
    pub(crate) fn peek_optional_tuple_element(&self, close: TokenType) -> bool {
        if !self.peek_is(TokenType::Maybe) {
            return false;
        }

        let next = self.peek_next_token_type();

        next == TokenType::Comma || next == close
    }

    /// Parse one labeled type tuple head.
    ///
    /// Examples:
    /// ```tspp
    /// name: string
    /// name?: string
    /// rest: ...string[]
    /// ```
    fn parse_tuple_label(&mut self) -> ParserResult<TupleLabel> {
        let (name, range) = self.eat_identifier_with_range()?;

        // parse the optional marker
        let is_optional = if self.peek_is(TokenType::Maybe) {
            self.bump();
            true
        } else {
            false
        };

        // close the label
        self.eat_token(TokenType::Colon)?;

        Ok(TupleLabel {
            name,
            range,
            is_optional,
        })
    }

    /// Parse one type tuple element directly in type space.
    ///
    /// Examples:
    /// ```tspp
    /// string
    /// name?: string
    /// ...rest: string[]
    /// ```
    fn parse_type_tuple_element(
        &mut self,
        stop: TypeStop,
        close: TokenType,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        let documentation = self.parse_documentation();
        let start = self.mark_parse_start();

        // parse a spread element
        let element = if self.peek_is(TokenType::Spread) {
            self.bump();
            let label = if self.peek_labeled_tuple() {
                Some(self.parse_tuple_label()?)
            } else {
                None
            };

            self.parse_tuple_rest_element(&start, label, stop)?
        }
        // parse a regular or labeled rest element
        else {
            let label = if self.peek_labeled_tuple() {
                Some(self.parse_tuple_label()?)
            } else {
                None
            };
            if label.is_some() && self.peek_is(TokenType::Spread) {
                self.bump();
                self.parse_tuple_rest_element(&start, label, stop)?
            } else {
                self.parse_tuple_element(&start, label, stop, close)?
            }
        };
        self.attach_documentation(element, documentation);

        Ok(element)
    }

    /// Parse one tuple rest element after its spread token.
    fn parse_tuple_rest_element(
        &mut self,
        start: &ParseStart,
        label: Option<TupleLabel>,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        // reject optional rest labels
        if label.is_some_and(|label| label.is_optional) {
            return Err(ParserError::unexpected(self.range_since(start)));
        }

        // parse the rest payload
        let value = self.parse_type(TypePosition::Type, stop.nest())?;

        let element = self.insert_node(
            TupleElement::Spread {
                label: label.map(|label| label.name),
                value,
            },
            self.range_since(start),
        );
        if let Some(label) = label {
            self.tree.set_main_range(element, label.range);
        }

        Ok(element)
    }

    /// Parse a regular tuple element.
    ///
    /// Examples:
    /// ```tspp
    /// string
    /// string?
    /// name?: string
    /// ```
    fn parse_tuple_element(
        &mut self,
        start: &ParseStart,
        label: Option<TupleLabel>,
        stop: TypeStop,
        close: TokenType,
    ) -> ParserResult<LocalNodeId<TupleElement>> {
        let value = self.parse_type(TypePosition::Type, stop.nest())?;

        let is_optional = if let Some(label) = label {
            label.is_optional
        } else if self.peek_optional_tuple_element(close) {
            self.bump();
            true
        } else {
            false
        };

        let element = self.insert_node(
            TupleElement::Element {
                label: label.map(|label| label.name),
                value,
                is_optional,
                is_readonly: false,
            },
            self.range_since(start),
        );
        if let Some(label) = label {
            self.tree.set_main_range(element, label.range);
        }

        Ok(element)
    }

    /// Parse the rest of a type tuple after one unlabeled value head.
    ///
    /// Examples:
    /// ```tspp
    /// string,
    /// string, number
    /// string?, ...boolean[]
    /// ```
    pub(crate) fn parse_type_tuple_tail(
        &mut self,
        start: &ParseStart,
        value: LocalNodeId<TypeExpression>,
        stop: TypeStop,
        close: TokenType,
    ) -> ParserResult<Vec<LocalNodeId<TupleElement>>> {
        // optional marker on an unlabeled tuple element
        let is_optional = if self.peek_optional_tuple_element(close) {
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
        if let Some(documentation) = self.tree.take_documentation(value.id) {
            self.tree.set_documentation(first_element.id, documentation);
        }

        // remaining elements
        let mut elements = vec![first_element];
        if self.peek_is(TokenType::Comma) {
            self.eat_token(TokenType::Comma)?;
            elements.extend(self.parse_type_tuple_elements(stop, close)?);
        }

        Ok(elements)
    }

    /// Parse type tuple elements until one closing token.
    ///
    /// Examples:
    /// ```tspp
    /// string, number
    /// name: string, age?: number
    /// ...rest: string[]
    /// ```
    pub(crate) fn parse_type_tuple_elements(
        &mut self,
        stop: TypeStop,
        close: TokenType,
    ) -> ParserResult<Vec<LocalNodeId<TupleElement>>> {
        let mut elements = Vec::new();

        while self.has_more_tokens() {
            // closing token
            if self.peek_is(close) {
                break;
            }

            // one tuple element
            let element_start = self.mark_parse_start();
            let (element, is_recovered) = match self.parse_type_tuple_element(stop, close) {
                Ok(element_id) => {
                    let is_recovered = matches!(self.tree.get(element_id), TupleElement::Error);

                    (element_id, is_recovered)
                }
                Err(error) => {
                    self.recover_list_item(self.range_since(&element_start), close, error);
                    let element_id =
                        self.insert_node(TupleElement::Error, self.range_since(&element_start));

                    (element_id, true)
                }
            };

            elements.push(element);

            // separator
            if self.peek_is(TokenType::Comma) {
                self.eat_token(TokenType::Comma)?;
            }
            // recovery boundary
            else if !is_recovered || !self.peek_recovered_list_continuation(close) {
                break;
            }
        }

        Ok(elements)
    }
}
