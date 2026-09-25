use crate::parse::r#type::operator::{TypeOperator, TypeRelation};
use crate::{Parser, ParserResult};
use tspp_dir::{LocalNodeId, TypeExpression};
use tspp_source::ByteRange;

impl Parser {
    /// Insert one type infix expression.
    pub(in crate::parse) fn insert_type_infix(
        &mut self,
        left: LocalNodeId<TypeExpression>,
        operator: TypeOperator,
        operator_range: ByteRange,
        right: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // cover the complete operation
        let left_range = self.tree.get_range(left);
        let right_range = self.tree.get_range(right);
        let source_range = ByteRange {
            start: left_range.start,
            end: right_range.end,
        };

        // extend one existing associative chain
        if matches!(operator, TypeOperator::Union | TypeOperator::Intersection)
            && self.extend_type_chain(left, right, operator, source_range)
        {
            return Ok(left);
        }

        // materialize every other operation
        let ty = match operator {
            TypeOperator::Union => TypeExpression::Union {
                elements: vec![left, right],
            },
            TypeOperator::Intersection => TypeExpression::Intersection {
                elements: vec![left, right],
            },
            TypeOperator::Relation(TypeRelation::Extends) => {
                TypeExpression::Extends { left, right }
            }
            TypeOperator::Relation(TypeRelation::Implements) => {
                TypeExpression::Implements { left, right }
            }
            TypeOperator::Range(end_kind) => TypeExpression::Range {
                start: Some(left),
                end: Some(right),
                end_kind,
            },
        };
        let ty = self.insert_node(ty, source_range);

        // retain operator and left-head source regions
        self.tree.set_main_range(ty, operator_range);
        self.tree
            .set_head_range(ty, self.type_expression_head_range(left));

        Ok(ty)
    }

    /// Extend one existing union or intersection expression.
    fn extend_type_chain(
        &mut self,
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
        operator: TypeOperator,
        source_range: ByteRange,
    ) -> bool {
        // select an existing chain of the same operation
        let elements = match (operator, self.tree.get_mut(left)) {
            (TypeOperator::Union, TypeExpression::Union { elements })
            | (TypeOperator::Intersection, TypeExpression::Intersection { elements }) => elements,
            _ => return false,
        };

        // append the operand and extend the existing node
        elements.push(right);
        self.tree.set_range(left, source_range);

        true
    }
}
