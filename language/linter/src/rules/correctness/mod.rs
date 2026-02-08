mod for_direction;
mod no_approx_constant;
mod no_array_constructor;
mod no_array_delete;
mod no_async_promise_executor;
mod no_base_to_string;
mod no_compare_neg_zero;
mod no_constant_binary_expression;
mod no_constant_condition;
mod no_control_regex;
mod no_duplicate_case;
mod no_empty_range;
mod no_fallthrough;
mod no_floating_point_equality;
mod no_floating_promises;
mod no_for_in_array;
mod no_infinite_recursion;
mod no_invalid_regexp;
mod no_iterator_invalidation;
mod no_loop_single_iteration;
mod no_misused_promises;
mod no_promise_executor_return;
mod no_self_compare;
mod no_sparse_arrays;
mod no_struct_identity_compare;
mod no_throw_in_result_function;
mod no_unknown_rule_decorator;
mod no_unsafe_finally;
mod no_unsafe_negation;
mod no_useless_assignment;
mod no_useless_increment;
mod require_array_sort_compare;
mod use_isnan;

use crate::{BoxedLintRule, boxed};

pub use for_direction::*;
pub use no_approx_constant::*;
pub use no_array_constructor::*;
pub use no_array_delete::*;
pub use no_async_promise_executor::*;
pub use no_base_to_string::*;
pub use no_compare_neg_zero::*;
pub use no_constant_binary_expression::*;
pub use no_constant_condition::*;
pub use no_control_regex::*;
pub use no_duplicate_case::*;
pub use no_empty_range::*;
pub use no_fallthrough::*;
pub use no_floating_point_equality::*;
pub use no_floating_promises::*;
pub use no_for_in_array::*;
pub use no_infinite_recursion::*;
pub use no_invalid_regexp::*;
pub use no_iterator_invalidation::*;
pub use no_loop_single_iteration::*;
pub use no_misused_promises::*;
pub use no_promise_executor_return::*;
pub use no_self_compare::*;
pub use no_sparse_arrays::*;
pub use no_struct_identity_compare::*;
pub use no_throw_in_result_function::*;
pub use no_unknown_rule_decorator::*;
pub use no_unsafe_finally::*;
pub use no_unsafe_negation::*;
pub use no_useless_assignment::*;
pub use no_useless_increment::*;
pub use require_array_sort_compare::*;
pub use use_isnan::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(ForDirection),
        boxed(NoApproxConstant),
        boxed(NoArrayConstructor),
        boxed(NoArrayDelete),
        boxed(NoAsyncPromiseExecutor),
        boxed(NoBaseToString),
        boxed(NoCompareNegZero),
        boxed(NoConstantBinaryExpression),
        boxed(NoConstantCondition),
        boxed(NoControlRegex),
        boxed(NoDuplicateCase),
        boxed(NoEmptyRange),
        boxed(NoFallthrough),
        boxed(NoFloatingPromises),
        boxed(NoFloatingPointEquality),
        boxed(NoForInArray),
        boxed(NoInfiniteRecursion),
        boxed(NoInvalidRegexp),
        boxed(NoIteratorInvalidation),
        boxed(NoLoopSingleIteration),
        boxed(NoMisusedPromises),
        boxed(NoPromiseExecutorReturn),
        boxed(NoSelfCompare),
        boxed(NoSparseArrays),
        boxed(NoStructIdentityCompare),
        boxed(NoThrowInResultFunction),
        boxed(NoUnknownRuleDecorator),
        boxed(NoUnsafeFinally),
        boxed(NoUnsafeNegation),
        boxed(NoUselessAssignment),
        boxed(NoUselessIncrement),
        boxed(RequireArraySortCompare),
        boxed(UseIsnan),
    ]
}
