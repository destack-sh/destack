mod for_direction;
mod no_approx_constant;
mod no_array_constructor;
mod no_array_delete;
mod no_async_promise_executor;
mod no_compare_neg_zero;
mod no_constant_binary_expression;
mod no_constant_condition;
mod no_control_regex;
mod no_duplicate_case;
mod no_empty_range;
mod no_fallthrough;
mod no_invalid_regexp;
mod no_loop_single_iteration;
mod no_promise_executor_return;
mod no_self_compare;
mod no_sparse_arrays;
mod no_unknown_rule_decorator;
mod no_unsafe_finally;
mod no_unsafe_negation;
mod use_isnan;

use crate::{BoxedLintRule, boxed};

pub use for_direction::*;
pub use no_approx_constant::*;
pub use no_array_constructor::*;
pub use no_array_delete::*;
pub use no_async_promise_executor::*;
pub use no_compare_neg_zero::*;
pub use no_constant_binary_expression::*;
pub use no_constant_condition::*;
pub use no_control_regex::*;
pub use no_duplicate_case::*;
pub use no_empty_range::*;
pub use no_fallthrough::*;
pub use no_invalid_regexp::*;
pub use no_loop_single_iteration::*;
pub use no_promise_executor_return::*;
pub use no_self_compare::*;
pub use no_sparse_arrays::*;
pub use no_unknown_rule_decorator::*;
pub use no_unsafe_finally::*;
pub use no_unsafe_negation::*;
pub use use_isnan::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(ForDirection),
        boxed(NoApproxConstant),
        boxed(NoArrayConstructor),
        boxed(NoArrayDelete),
        boxed(NoAsyncPromiseExecutor),
        boxed(NoCompareNegZero),
        boxed(NoControlRegex),
        boxed(NoConstantBinaryExpression),
        boxed(NoConstantCondition),
        boxed(NoDuplicateCase),
        boxed(NoEmptyRange),
        boxed(NoInvalidRegexp),
        boxed(NoFallthrough),
        boxed(NoLoopSingleIteration),
        boxed(NoPromiseExecutorReturn),
        boxed(NoSelfCompare),
        boxed(NoSparseArrays),
        boxed(NoUnknownRuleDecorator),
        boxed(NoUnsafeFinally),
        boxed(NoUnsafeNegation),
        boxed(UseIsnan),
    ]
}
