use crate::{ParseError, ParseResult, Parser};
use destack_dir::{
    BinaryOperator, Keyword, LocalNodeId, OperatorPrecedence, TokenType, TypeExpression,
    TypePredicateSubject,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// Type-space unary operators.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TypeUnaryOperator {
    /// `keyof T`.
    Keyof,
    /// `readonly T`.
    Readonly,
    /// `local T`.
    Local,
    /// `shared T`.
    Shared,
    /// `!T`.
    Not,
}

impl TypeUnaryOperator {
    /// Return this operator precedence.
    pub(crate) fn precedence(self) -> u16 {
        OperatorPrecedence::Prefix as u16
    }
}

/// Type-space relation operators.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TypeBinaryOperator {
    /// `extends`.
    Extends,
    /// `implements`.
    Implements,
}

impl TypeBinaryOperator {
    /// Return this operator precedence.
    pub(crate) fn precedence(self) -> u16 {
        match self {
            Self::Extends | Self::Implements => OperatorPrecedence::Comparison as u16,
        }
    }
}

/// One type expression infix operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(in crate::parse) enum TypeInfixOperator {
    /// `|` or `&`.
    Binary(BinaryOperator),
    /// `extends` or `implements`.
    Relation(TypeBinaryOperator),
    /// Type predicate relation.
    Is,
    /// Destack range operator.
    Range(destack_dir::RangeEnd),
}

impl TypeInfixOperator {
    /// Return this operator precedence.
    pub(in crate::parse) fn precedence(self) -> u16 {
        match self {
            Self::Binary(operator) => operator.precedence(),
            Self::Relation(operator) => operator.precedence(),
            Self::Is => OperatorPrecedence::Comparison as u16,
            Self::Range(_) => OperatorPrecedence::Range as u16,
        }
    }
}

impl Parser {
    /// Return a type prefix operator at the current token.
    pub(crate) fn peek_type_unary_prefix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        match self.peek_token_type() {
            TokenType::Not => Some(TypeUnaryOperator::Not),
            TokenType::Identifier => match self.current_keyword()? {
                Keyword::Keyof => Some(TypeUnaryOperator::Keyof),
                Keyword::Readonly => Some(TypeUnaryOperator::Readonly),
                Keyword::Local if self.language.is_destack() => Some(TypeUnaryOperator::Local),
                Keyword::Shared if self.language.is_destack() => Some(TypeUnaryOperator::Shared),
                _ => None,
            },
            _ => None,
        }
    }

    /// Build a type expression for one infix operator.
    pub(in crate::parse) fn make_type_infix_expression(
        &mut self,
        source_span: Span,
        head_span: Span,
        left: LocalNodeId<TypeExpression>,
        operator: TypeInfixOperator,
        operator_span: Span,
        right: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let expression = match operator {
            TypeInfixOperator::Binary(BinaryOperator::ElementwiseOr) => {
                if self.extend_type_binary_expression(source_span, left, right, true) {
                    return Ok(left);
                }

                TypeExpression::Union {
                    elements: self.flatten_type_binary_elements(left, right, true),
                }
            }
            TypeInfixOperator::Binary(BinaryOperator::ElementwiseAnd) => {
                if self.extend_type_binary_expression(source_span, left, right, false) {
                    return Ok(left);
                }

                TypeExpression::Intersection {
                    elements: self.flatten_type_binary_elements(left, right, false),
                }
            }
            TypeInfixOperator::Relation(TypeBinaryOperator::Extends) => {
                TypeExpression::Extends { left, right }
            }
            TypeInfixOperator::Relation(TypeBinaryOperator::Implements) => {
                TypeExpression::Implements { left, right }
            }
            TypeInfixOperator::Is => {
                let Some(subject) = self.type_predicate_subject_from_type_expression(left) else {
                    return Err(ParseError::unexpected(self.tree.get_span(left)));
                };
                self.set_node_leading_span(right, operator_span.end);
                let id = self.insert_node(
                    TypeExpression::Predicate {
                        asserts: false,
                        subject,
                        target: Some(right),
                    },
                    source_span,
                );
                let subject_span = self
                    .tree
                    .get_main_span(left)
                    .unwrap_or_else(|| self.tree.get_span(left));
                self.tree.set_main_span(id, subject_span);
                self.tree.set_head_span(id, head_span);
                self.tree.set_side_span(
                    id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    operator_span,
                );
                return Ok(id);
            }
            _ => return Err(ParseError::unexpected(self.tree.get_span(right))),
        };

        let id = self.insert_node(expression, source_span);
        self.tree.set_head_span(id, head_span);

        Ok(id)
    }

    /// Return one type predicate subject from a type expression.
    pub(crate) fn type_predicate_subject_from_type_expression(
        &self,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<TypePredicateSubject> {
        match self.tree.get(expression_id) {
            TypeExpression::Reference {
                path,
                generic_arguments,
            } if generic_arguments.is_empty() && path.segments.len() == 1 => {
                Some(TypePredicateSubject::Identifier(path.segments[0]))
            }
            TypeExpression::This => Some(TypePredicateSubject::This),
            _ => None,
        }
    }

    /// Return true when a type expression is bare `this`.
    pub(crate) fn type_expression_is_bare_this(
        &self,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        matches!(self.tree.get(expression_id), TypeExpression::This)
    }

    /// Return flattened type binary elements for chain nodes.
    fn flatten_type_binary_elements(
        &self,
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
        is_union: bool,
    ) -> Vec<LocalNodeId<TypeExpression>> {
        let mut elements = Vec::new();
        match self.tree.get(left) {
            TypeExpression::Union {
                elements: left_elements,
            } if is_union => {
                elements.extend(left_elements.iter().copied());
            }
            TypeExpression::Intersection {
                elements: left_elements,
            } if !is_union => {
                elements.extend(left_elements.iter().copied());
            }
            _ => elements.push(left),
        }

        match self.tree.get(right) {
            TypeExpression::Union {
                elements: right_elements,
            } if is_union => {
                elements.extend(right_elements.iter().copied());
            }
            TypeExpression::Intersection {
                elements: right_elements,
            } if !is_union => {
                elements.extend(right_elements.iter().copied());
            }
            _ => elements.push(right),
        }

        elements
    }

    /// Extend an active union or intersection node in place.
    fn extend_type_binary_expression(
        &mut self,
        source_span: Span,
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
        is_union: bool,
    ) -> bool {
        let right_elements = self.right_type_binary_elements(right, is_union);
        let left_expression = self.tree.get_mut(left);

        let elements = match left_expression {
            TypeExpression::Union { elements } if is_union => elements,
            TypeExpression::Intersection { elements } if !is_union => elements,
            _ => return false,
        };

        if let Some(right_elements) = right_elements {
            elements.extend(right_elements);
        } else {
            elements.push(right);
        }
        self.tree.set_span(left, source_span);

        true
    }

    /// Take right elements when a grouped right side already matches the operator.
    fn right_type_binary_elements(
        &mut self,
        right: LocalNodeId<TypeExpression>,
        is_union: bool,
    ) -> Option<Vec<LocalNodeId<TypeExpression>>> {
        match self.tree.get_mut(right) {
            TypeExpression::Union { elements } if is_union => Some(std::mem::take(elements)),
            TypeExpression::Intersection { elements } if !is_union => {
                Some(std::mem::take(elements))
            }
            _ => None,
        }
    }
}
