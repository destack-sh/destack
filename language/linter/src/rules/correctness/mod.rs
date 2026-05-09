mod for_direction;
mod no_approx_constant;
mod no_arguments_order_mismatch;
mod no_async_promise_executor;
mod no_base_to_string;
mod no_compare_neg_zero;
mod no_confusing_void_expression;
mod no_constant_binary_expression;
mod no_constant_condition;
mod no_control_regex;
mod no_deprecated;
mod no_duplicate_case;
mod no_fallthrough;
mod no_floating_point_equality;
mod no_floating_promises;
mod no_for_in_array;
mod no_infinite_recursion;
mod no_invalid_regexp;
mod no_iterator_invalidation;
mod no_loop_single_iteration;
mod no_misused_promises;
mod no_overlapping_match_arms;
mod no_promise_executor_return;
mod no_self_compare;
mod no_struct_identity_compare;
mod no_throw_in_result_function;
mod no_unknown_rule_decorator;
mod no_unnecessary_condition;
mod no_unnecessary_type_arguments;
mod no_unnecessary_type_assertion;
mod no_unsafe_finally;
mod no_unsafe_negation;
mod no_unused_imports;
mod no_unused_parameters;
mod no_unused_private_class_members;
mod no_useless_assignment;
mod no_useless_increment;
mod require_array_sort_compare;
mod unbound_method;
mod unused_must_use;
mod use_isnan;

use crate::{BoxedLintRule, boxed};

pub use for_direction::*;
pub use no_approx_constant::*;
pub use no_arguments_order_mismatch::*;
pub use no_async_promise_executor::*;
pub use no_base_to_string::*;
pub use no_compare_neg_zero::*;
pub use no_confusing_void_expression::*;
pub use no_constant_binary_expression::*;
pub use no_constant_condition::*;
pub use no_control_regex::*;
pub use no_deprecated::*;
pub use no_duplicate_case::*;
pub use no_fallthrough::*;
pub use no_floating_point_equality::*;
pub use no_floating_promises::*;
pub use no_for_in_array::*;
pub use no_infinite_recursion::*;
pub use no_invalid_regexp::*;
pub use no_iterator_invalidation::*;
pub use no_loop_single_iteration::*;
pub use no_misused_promises::*;
pub use no_overlapping_match_arms::*;
pub use no_promise_executor_return::*;
pub use no_self_compare::*;
pub use no_struct_identity_compare::*;
pub use no_throw_in_result_function::*;
pub use no_unknown_rule_decorator::*;
pub use no_unnecessary_condition::*;
pub use no_unnecessary_type_arguments::*;
pub use no_unnecessary_type_assertion::*;
pub use no_unsafe_finally::*;
pub use no_unsafe_negation::*;
pub use no_unused_imports::*;
pub use no_unused_parameters::*;
pub use no_unused_private_class_members::*;
pub use no_useless_assignment::*;
pub use no_useless_increment::*;
pub use require_array_sort_compare::*;
pub use unbound_method::*;
pub use unused_must_use::*;
pub use use_isnan::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(ForDirection),
        boxed(NoApproxConstant),
        boxed(NoArgumentsOrderMismatch),
        boxed(NoAsyncPromiseExecutor),
        boxed(NoBaseToString),
        boxed(NoCompareNegZero),
        boxed(NoConfusingVoidExpression),
        boxed(NoConstantBinaryExpression),
        boxed(NoConstantCondition),
        boxed(NoControlRegex),
        boxed(NoDeprecated),
        boxed(NoDuplicateCase),
        boxed(NoFallthrough),
        boxed(NoFloatingPromises),
        boxed(NoFloatingPointEquality),
        boxed(NoForInArray),
        boxed(NoInfiniteRecursion),
        boxed(NoInvalidRegexp),
        boxed(NoIteratorInvalidation),
        boxed(NoLoopSingleIteration),
        boxed(NoMisusedPromises),
        boxed(NoOverlappingMatchArms),
        boxed(NoPromiseExecutorReturn),
        boxed(NoSelfCompare),
        boxed(NoStructIdentityCompare),
        boxed(NoThrowInResultFunction),
        boxed(NoUnknownRuleDecorator),
        boxed(NoUnsafeFinally),
        boxed(NoUnsafeNegation),
        boxed(NoUnusedImports),
        boxed(NoUnusedParameters),
        boxed(NoUnusedPrivateClassMembers),
        boxed(NoUnnecessaryTypeAssertion),
        boxed(NoUnnecessaryTypeArguments),
        boxed(NoUnnecessaryCondition),
        boxed(NoUselessAssignment),
        boxed(NoUselessIncrement),
        boxed(RequireArraySortCompare),
        boxed(UnboundMethod),
        boxed(UnusedMustUse),
        boxed(UseIsnan),
    ]
}
