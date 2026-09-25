use crate::Parser;
use tspp_dir::{Expression, LocalNodeId, TypeExpression};
use tspp_source::ByteRange;

impl Parser {
    /// Return the source head range of one type expression.
    pub(crate) fn type_expression_head_range(&self, ty: LocalNodeId<TypeExpression>) -> ByteRange {
        if let Some(range) = self.tree.get_head_range(ty) {
            return range;
        }

        if let Some(range) = self.tree.get_main_range(ty) {
            return range;
        }

        self.tree.get_range(ty)
    }

    /// Return the source head range of one value expression.
    pub(crate) fn expression_head_range(&self, expression: LocalNodeId<Expression>) -> ByteRange {
        if let Some(range) = self.tree.get_head_range(expression) {
            return range;
        }

        match self.tree.get(expression) {
            Expression::Type { value } => self.type_expression_head_range(*value),
            Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                self.expression_head_range(*expression)
            }
            _ => match self.tree.get_main_range(expression) {
                Some(range) => range,
                None => self.tree.get_range(expression),
            },
        }
    }
}
