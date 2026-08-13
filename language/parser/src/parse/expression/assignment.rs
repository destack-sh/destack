use crate::{Parser, ParserError, ParserResult};
use destack_core::ensure_sufficient_stack;
use destack_dir::{
    Argument, AssignOperator, AssignPattern, AssignPatternField, Expression, Key, LocalNodeId,
    Property, UnaryOperator,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Lower one value expression into an assignment target.
    pub(in crate::parse) fn lower_assignment_target(
        &mut self,
        expression: LocalNodeId<Expression>,
        operator: AssignOperator,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let is_destructuring = matches!(
            self.tree.get(expression),
            Expression::ArrayExpression { .. }
                | Expression::TupleExpression { .. }
                | Expression::ObjectExpression { .. }
        );
        let mark = self.tree.mark();
        let target = if is_destructuring {
            ensure_sufficient_stack(|| self.lower_assignment_pattern(expression))
        } else {
            self.lower_assignment_pattern(expression)
        };
        let target = match target {
            Ok(target) => target,
            Err(error) => {
                self.tree.restore_to_mark(mark);

                return Err(error);
            }
        };

        // compound assignment requires one writable place
        if operator != AssignOperator::Assign
            && !matches!(self.tree.get(target), AssignPattern::Place { .. })
        {
            let range = self.tree.get_range(target);
            self.tree.restore_to_mark(mark);

            return Err(ParserError::invalid_assignment_target(range));
        }

        Ok(target)
    }

    /// Lower one assignment target at the current stack depth.
    fn lower_assignment_pattern(
        &mut self,
        expression: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let has_parentheses = self
            .tree
            .get_side_range(
                expression,
                NodeSpanType::Region(NodeSpanRegion::Parentheses),
            )
            .is_some();

        // assertions must be parenthesized before assignment
        if matches!(
            self.tree.get(expression),
            Expression::As { .. } | Expression::Satisfies { .. }
        ) && !has_parentheses
        {
            return Err(ParserError::invalid_assignment_target(
                self.tree.get_range(expression),
            ));
        }

        // assignments and destructuring cannot be hidden by parentheses
        if has_parentheses
            && matches!(
                self.tree.get(expression),
                Expression::Assign { .. }
                    | Expression::ArrayExpression { .. }
                    | Expression::TupleExpression { .. }
                    | Expression::ObjectExpression { .. }
            )
        {
            return Err(ParserError::invalid_assignment_target(
                self.tree.get_range(expression),
            ));
        }

        // lower array destructuring recursively
        if let Expression::ArrayExpression { elements } = self.tree.get(expression) {
            let elements = elements.clone();

            return self.lower_sequence_assignment(expression, elements);
        }

        // lower tuple destructuring recursively
        if let Expression::TupleExpression { elements } = self.tree.get(expression) {
            let elements = elements.clone();

            return self.lower_tuple_assignment(expression, elements);
        }

        // lower object destructuring recursively
        if let Expression::ObjectExpression { properties } = self.tree.get(expression) {
            let properties = properties.clone();

            return self.lower_object_assignment(expression, properties);
        }

        // lower default values into assignment patterns
        if let Expression::Assign {
            left,
            operator,
            right,
        } = self.tree.get(expression)
        {
            let left = *left;
            let operator = *operator;
            let right = *right;
            let range = self.tree.get_range(expression);

            if operator != AssignOperator::Assign {
                return Err(ParserError::invalid_assignment_target(range));
            }

            let pattern = AssignPattern::Default {
                pattern: left,
                value: right,
            };

            return Ok(self.insert_node(pattern, range));
        }

        // every remaining target must denote one writable place
        if !self.is_assignment_place(expression) {
            return Err(ParserError::invalid_assignment_target(
                self.tree.get_range(expression),
            ));
        }

        Ok(self.insert_node(
            AssignPattern::Place { expression },
            self.tree.get_range(expression),
        ))
    }

    /// Lower one array expression into a sequence assignment target.
    fn lower_sequence_assignment(
        &mut self,
        expression: LocalNodeId<Expression>,
        elements: Vec<LocalNodeId<Argument>>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let fields = elements
            .into_iter()
            .map(|argument| self.lower_sequence_assignment_field(argument))
            .collect::<ParserResult<Vec<_>>>()?;
        let range = self.tree.get_range(expression);

        Ok(self.insert_node(AssignPattern::Sequence { fields }, range))
    }

    /// Lower one array element into a sequence assignment field.
    fn lower_sequence_assignment_field(
        &mut self,
        argument: LocalNodeId<Argument>,
    ) -> ParserResult<LocalNodeId<AssignPatternField>> {
        let node = self.tree.get(argument).clone();
        let range = self.tree.get_range(argument);
        let field = match node {
            Argument::Elision => AssignPatternField::Elision,
            Argument::Positional { value } => AssignPatternField::Positional {
                pattern: self.lower_assignment_pattern(value)?,
            },
            Argument::Spread { value } => AssignPatternField::Rest {
                pattern: Some(self.lower_assignment_pattern(value)?),
            },
            Argument::Error => {
                return Err(ParserError::invalid_assignment_target(range));
            }
        };

        Ok(self.insert_node(field, range))
    }

    /// Lower one tuple expression into a tuple assignment target.
    fn lower_tuple_assignment(
        &mut self,
        expression: LocalNodeId<Expression>,
        elements: Vec<LocalNodeId<Argument>>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let fields = elements
            .into_iter()
            .map(|argument| self.lower_tuple_assignment_field(argument))
            .collect::<ParserResult<Vec<_>>>()?;
        let range = self.tree.get_range(expression);

        Ok(self.insert_node(AssignPattern::Tuple { fields }, range))
    }

    /// Lower one tuple element into a tuple assignment field.
    fn lower_tuple_assignment_field(
        &mut self,
        argument: LocalNodeId<Argument>,
    ) -> ParserResult<LocalNodeId<AssignPatternField>> {
        let node = self.tree.get(argument).clone();
        let range = self.tree.get_range(argument);
        let Argument::Positional { value } = node else {
            return Err(ParserError::invalid_assignment_target(range));
        };
        let pattern = self.lower_assignment_pattern(value)?;

        Ok(self.insert_node(AssignPatternField::Positional { pattern }, range))
    }

    /// Lower one object expression into an object assignment target.
    fn lower_object_assignment(
        &mut self,
        expression: LocalNodeId<Expression>,
        properties: Vec<LocalNodeId<Property>>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let fields = properties
            .into_iter()
            .map(|property| self.lower_object_assignment_field(property))
            .collect::<ParserResult<Vec<_>>>()?;
        let range = self.tree.get_range(expression);

        Ok(self.insert_node(AssignPattern::Object { fields }, range))
    }

    /// Lower one object property into an object assignment field.
    fn lower_object_assignment_field(
        &mut self,
        property: LocalNodeId<Property>,
    ) -> ParserResult<LocalNodeId<AssignPatternField>> {
        let node = self.tree.get(property).clone();
        let range = self.tree.get_range(property);
        let main_range = self.tree.get_main_range(property);
        let field = match node {
            Property::Field {
                key: Key::Name(name),
                value,
                is_shorthand,
            } => {
                let pattern = self.lower_assignment_pattern(value)?;

                AssignPatternField::Named {
                    name,
                    pattern,
                    is_shorthand,
                }
            }
            Property::Field {
                key: Key::Expression(key),
                value,
                ..
            } => AssignPatternField::Computed {
                key,
                pattern: self.lower_assignment_pattern(value)?,
            },
            Property::Spread { value } => AssignPatternField::Rest {
                pattern: Some(self.lower_assignment_pattern(value)?),
            },
            Property::Method { .. } | Property::Error => {
                return Err(ParserError::invalid_assignment_target(range));
            }
        };

        // preserve the property key as the assignment field's main range
        let field = self.insert_node(field, range);
        if let Some(main_range) = main_range {
            self.tree.set_main_range(field, main_range);
        }

        Ok(field)
    }

    /// Return whether one expression denotes a writable place.
    fn is_assignment_place(&self, mut expression: LocalNodeId<Expression>) -> bool {
        loop {
            // accept direct writable places
            match self.tree.get(expression) {
                Expression::Identifier { .. } => return true,
                Expression::Member { .. } | Expression::Index { .. } => {
                    return !self.contains_optional_chain(expression);
                }
                Expression::As {
                    expression: left, ..
                }
                | Expression::Satisfies {
                    expression: left, ..
                } => expression = *left,
                Expression::Unary {
                    operator: UnaryOperator::Dereference,
                    right,
                } => expression = *right,
                Expression::Must { left, .. } => expression = *left,
                _ => return false,
            }
        }
    }

    /// Return whether one expression contains optional chaining.
    fn contains_optional_chain(&self, mut expression: LocalNodeId<Expression>) -> bool {
        loop {
            // follow the left edge of the place expression
            match self.tree.get(expression) {
                Expression::Maybe { .. } => return true,
                Expression::Member { left, .. } | Expression::Index { left, .. } => {
                    expression = *left;
                }
                Expression::As {
                    expression: left, ..
                }
                | Expression::Satisfies {
                    expression: left, ..
                } => expression = *left,
                Expression::Must { left, .. } => expression = *left,
                _ => return false,
            }
        }
    }
}
