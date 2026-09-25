use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::DirModule;

/// One range represented by a pair of ordered comparisons.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ComparisonRange {
    /// The repeated value.
    pub(crate) value: dir::LocalNodeId<dir::Expression>,
    /// The included lower bound.
    pub(crate) start: dir::LocalNodeId<dir::Expression>,
    /// The upper bound.
    pub(crate) end: dir::LocalNodeId<dir::Expression>,
    /// Whether the upper bound is included.
    pub(crate) end_kind: dir::RangeEnd,
    /// Whether the comparisons test values outside the range.
    pub(crate) is_negated: bool,
}

/// One comparison against a lower or upper bound.
#[derive(Debug, Clone, Copy)]
struct BoundComparison {
    /// The repeated value.
    value: dir::LocalNodeId<dir::Expression>,
    /// The interval bound.
    bound: dir::LocalNodeId<dir::Expression>,
    /// The operator with the repeated value on its left.
    operator: dir::BinaryOperator,
}

/// One ordered binary comparison.
#[derive(Debug, Clone, Copy)]
struct OrderedComparison {
    /// The comparison operands.
    operands: [dir::LocalNodeId<dir::Expression>; 2],
    /// The comparison operator.
    operator: dir::BinaryOperator,
}

impl OrderedComparison {
    /// Place the selected value operand on the left.
    fn orient(self, is_value_right: bool) -> Option<BoundComparison> {
        let [left, right] = self.operands;
        let comparison = if is_value_right {
            BoundComparison {
                value: right,
                bound: left,
                operator: self.operator.swapped()?,
            }
        } else {
            BoundComparison {
                value: left,
                bound: right,
                operator: self.operator,
            }
        };

        Some(comparison)
    }
}

impl DirModule<'_> {
    /// Return the range represented by one pair of comparisons.
    pub(crate) fn comparison_range(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<ComparisonRange>, ProviderError> {
        let Some((logical, [left, right])) = self.builtin_binary(expression)? else {
            return Ok(None);
        };
        if !matches!(logical, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            return Ok(None);
        }
        let Some([left, right]) =
            self.align_range_comparisons(left.source.local_id, right.source.local_id)?
        else {
            return Ok(None);
        };

        // select the represented range and whether its membership is negated
        use dir::BinaryOperator::{
            And, GreaterThan, GreaterThanOrEqual, LessThan, LessThanOrEqual, Or,
        };
        use dir::RangeEnd::{Inclusive, Open};
        let (start, end, end_kind, is_negated) = match (logical, left.operator, right.operator) {
            (And, GreaterThanOrEqual, LessThan) => (left, right, Open, false),
            (And, LessThan, GreaterThanOrEqual) => (right, left, Open, false),
            (And, GreaterThanOrEqual, LessThanOrEqual) => (left, right, Inclusive, false),
            (And, LessThanOrEqual, GreaterThanOrEqual) => (right, left, Inclusive, false),
            (Or, LessThan, GreaterThanOrEqual) => (left, right, Open, true),
            (Or, GreaterThanOrEqual, LessThan) => (right, left, Open, true),
            (Or, LessThan, GreaterThan) => (left, right, Inclusive, true),
            (Or, GreaterThan, LessThan) => (right, left, Inclusive, true),
            _ => return Ok(None),
        };

        // preserve NaN behavior in out-of-range floating-point disjunctions
        let value_domain = self.node_type(start.value.into_any())?.scalar_domain();
        if is_negated && value_domain == Some(dir::ScalarDomain::Float) {
            return Ok(None);
        }

        // require compatible speculatable expressions
        let start_domain = self.node_type(start.bound.into_any())?.scalar_domain();
        let end_domain = self.node_type(end.bound.into_any())?.scalar_domain();
        if start_domain != end_domain
            || !self.is_speculatable_expression(start.value)?
            || !self.is_speculatable_expression(start.bound)?
            || !self.is_speculatable_expression(end.bound)?
        {
            return Ok(None);
        }

        Ok(Some(ComparisonRange {
            value: start.value,
            start: start.bound,
            end: end.bound,
            end_kind,
            is_negated,
        }))
    }

    /// Align two comparisons around their repeated operand.
    fn align_range_comparisons(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<[BoundComparison; 2]>, ProviderError> {
        let Some(left) = self.ordered_comparison(left)? else {
            return Ok(None);
        };
        let Some(right) = self.ordered_comparison(right)? else {
            return Ok(None);
        };

        // select the operand position repeated by both comparisons
        let [left_first, left_second] = left.operands;
        let [right_first, right_second] = right.operands;
        let orientation = if self.is_same_computation(left_first, right_first)? {
            (false, false)
        } else if self.is_same_computation(left_first, right_second)? {
            (false, true)
        } else if self.is_same_computation(left_second, right_first)? {
            (true, false)
        } else if self.is_same_computation(left_second, right_second)? {
            (true, true)
        } else {
            return Ok(None);
        };
        let (Some(left), Some(right)) = (left.orient(orientation.0), right.orient(orientation.1))
        else {
            return Ok(None);
        };

        Ok(Some([left, right]))
    }

    /// Select one builtin or canonical bigint ordering comparison.
    fn ordered_comparison(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<OrderedComparison>, ProviderError> {
        // recognize compiler-defined scalar comparisons
        if let Some((operator, operands)) = self.builtin_binary(expression)?
            && matches!(
                operator,
                dir::BinaryOperator::LessThan
                    | dir::BinaryOperator::LessThanOrEqual
                    | dir::BinaryOperator::GreaterThan
                    | dir::BinaryOperator::GreaterThanOrEqual
            )
        {
            let operands = [operands[0].source.local_id, operands[1].source.local_id];

            return Ok(Some(OrderedComparison { operands, operator }));
        }

        // recognize standard-library bigint comparisons
        let Some((operator, operands)) = self.integral_binary(expression)? else {
            return Ok(None);
        };
        if !matches!(
            operator,
            dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        ) {
            return Ok(None);
        }

        Ok(Some(OrderedComparison { operands, operator }))
    }
}
