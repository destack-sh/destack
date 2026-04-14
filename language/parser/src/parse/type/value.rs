use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Argument, Expression, LocalNodeId, StringId, TupleElement, TypeExpression};

impl Parser {
    /// Insert one positional argument node for an existing value.
    pub(crate) fn insert_positional_argument(
        &mut self,
        value: LocalNodeId<Expression>,
    ) -> LocalNodeId<Argument> {
        self.insert_node(Argument::Positional { value }, self.tree.get_span(value))
    }

    /// Wrap one type expression as a value-space type expression.
    pub(crate) fn insert_type_expression_value(
        &mut self,
        value: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<Expression> {
        // Expression::Type wrapper
        let expression_id = self.insert_node(Expression::Type { value }, self.tree.get_span(value));

        // preserve source spans
        if let Some(main_span) = self.tree.get_main_span(value) {
            self.tree.set_main_span(expression_id, main_span);
        }

        if let Some(head_span) = self.tree.get_head_span(value) {
            self.tree.set_head_span(expression_id, head_span);
        }

        expression_id
    }

    /// Return the embedded type expression when one expression is a type value wrapper.
    pub(crate) fn expression_type_value_maybe(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        match self.tree.get(expression_id) {
            Expression::Type { value } => Some(*value),
            _ => None,
        }
    }

    /// Return the embedded type expression or report an unexpected value-space expression.
    pub(crate) fn expect_type_expression_value(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        // unwrap the type wrapper
        match self.expression_type_value_maybe(expression_id) {
            Some(type_expression_id) => Ok(type_expression_id),
            None => Err(ParseError::unexpected(self.tree.get_span(expression_id))),
        }
    }

    /// Build one tuple element from one labeled or positional type value.
    fn build_tuple_element_from_value(
        &mut self,
        label: Option<StringId>,
        mut value: LocalNodeId<Expression>,
    ) -> ParseResult<TupleElement> {
        let mut is_optional = false;
        let mut is_readonly = false;

        // tuple element optionality is represented on the element, not the child type
        if let Expression::Maybe { left, .. } = self.tree.get(value) {
            is_optional = true;
            value = *left;
        }

        let mut value = self.expect_type_expression_value(value)?;

        // readonly tuple elements are also element-level state
        if let TypeExpression::Readonly { target_type } = self.tree.get(value) {
            is_readonly = true;
            value = *target_type;
        }

        Ok(TupleElement::Element {
            label,
            value,
            is_optional,
            is_readonly,
        })
    }

    /// Build one tuple element from one parsed type-space argument slot.
    fn build_tuple_element_from_argument(
        &mut self,
        argument_id: LocalNodeId<Argument>,
    ) -> ParseResult<LocalNodeId<TupleElement>> {
        // decouple the read from later insertion
        let argument = self.tree.get(argument_id).clone();

        // normalize the slot into one tuple element
        let element = match argument {
            Argument::Labeled { label, value } => {
                self.build_tuple_element_from_value(Some(label), value)?
            }
            Argument::Positional { value } => self.build_tuple_element_from_value(None, value)?,
            Argument::Spread { label, value } => {
                let value = self.expect_type_expression_value(value)?;

                TupleElement::Spread { label, value }
            }
            Argument::Named { .. } | Argument::Error => TupleElement::Error,
        };

        Ok(self.insert_node(element, self.tree.get_span(argument_id)))
    }

    /// Build tuple elements from parsed type-space argument slots.
    pub(crate) fn build_tuple_elements_from_arguments(
        &mut self,
        argument_ids: &[LocalNodeId<Argument>],
    ) -> ParseResult<Vec<LocalNodeId<TupleElement>>> {
        // build each element in source order
        let mut element_ids = Vec::with_capacity(argument_ids.len());

        for argument_id in argument_ids {
            let element_id = self.build_tuple_element_from_argument(*argument_id)?;
            element_ids.push(element_id);
        }

        Ok(element_ids)
    }

    /// Insert one type expression while preserving source spans from its wrapper.
    pub(crate) fn insert_wrapped_type_expression(
        &mut self,
        source_id: LocalNodeId<Expression>,
        expression: TypeExpression,
    ) -> LocalNodeId<TypeExpression> {
        // mirrored type node
        let expression_id = self.insert_node(expression, self.tree.get_span(source_id));

        // preserve source spans
        if let Some(main_span) = self.tree.get_main_span(source_id) {
            self.tree.set_main_span(expression_id, main_span);
        }

        if let Some(head_span) = self.tree.get_head_span(source_id) {
            self.tree.set_head_span(expression_id, head_span);
        }

        expression_id
    }
}
