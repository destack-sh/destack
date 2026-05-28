mod no_async_foreach;
mod no_cond_assign;
mod no_confusing_assignment;
mod no_confusing_non_null_assertion;
mod no_constant_assertion;
mod no_constructor_return;
mod no_debugger;
mod no_duplicate_decorators;
mod no_duplicate_else_if;
mod no_duplicate_match_arms;
mod no_empty;
mod no_empty_function;
mod no_empty_pattern;
mod no_empty_static_block;
mod no_extra_non_null_assertion;
mod no_identical_branches;
mod no_loop_func;
mod no_misleading_character_class;
mod no_mixed_key_types;
mod no_negation_in_equality_check;
mod no_redundant_match_guard;
mod no_redundant_pattern;
mod no_return_assign;
mod no_self_assign;
mod no_shadow_restricted_names;
mod no_single_element_tuple;
mod no_template_curly_in_string;
mod no_unused_except_recursion;
mod no_useless_backreference;
mod no_useless_computed_key;
mod no_useless_concat;
mod no_useless_constructor;
mod no_useless_escape;
mod no_useless_rename;
mod no_useless_return;
mod require_await;
mod require_else_in_if_chain;
mod require_yield;
mod return_await;

use crate::{BoxedLintRule, boxed};

pub use no_async_foreach::*;
pub use no_cond_assign::*;
pub use no_confusing_assignment::*;
pub use no_confusing_non_null_assertion::*;
pub use no_constant_assertion::*;
pub use no_constructor_return::*;
pub use no_debugger::*;
pub use no_duplicate_decorators::*;
pub use no_duplicate_else_if::*;
pub use no_duplicate_match_arms::*;
pub use no_empty::*;
pub use no_empty_function::*;
pub use no_empty_pattern::*;
pub use no_empty_static_block::*;
pub use no_extra_non_null_assertion::*;
pub use no_identical_branches::*;
pub use no_loop_func::*;
pub use no_misleading_character_class::*;
pub use no_mixed_key_types::*;
pub use no_negation_in_equality_check::*;
pub use no_redundant_match_guard::*;
pub use no_redundant_pattern::*;
pub use no_return_assign::*;
pub use no_self_assign::*;
pub use no_shadow_restricted_names::*;
pub use no_single_element_tuple::*;
pub use no_template_curly_in_string::*;
pub use no_unused_except_recursion::*;
pub use no_useless_backreference::*;
pub use no_useless_computed_key::*;
pub use no_useless_concat::*;
pub use no_useless_constructor::*;
pub use no_useless_escape::*;
pub use no_useless_rename::*;
pub use no_useless_return::*;
pub use require_await::*;
pub use require_else_in_if_chain::*;
pub use require_yield::*;
pub use return_await::*;

/// Get all suspicious rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoAsyncForeach),
        boxed(NoCondAssign),
        boxed(NoConfusingAssignment),
        boxed(NoConstantAssertion),
        boxed(NoConfusingNonNullAssertion),
        boxed(NoConstructorReturn),
        boxed(NoDebugger),
        boxed(NoDuplicateElseIf),
        boxed(NoDuplicateDecorators),
        boxed(NoDuplicateMatchArms),
        boxed(NoEmpty),
        boxed(NoEmptyFunction),
        boxed(NoEmptyPattern),
        boxed(NoEmptyStaticBlock),
        boxed(NoExtraNonNullAssertion),
        boxed(NoIdenticalBranches),
        boxed(NoLoopFunc),
        boxed(NoMisleadingCharacterClass),
        boxed(NoMixedKeyTypes),
        boxed(NoNegationInEqualityCheck),
        boxed(NoRedundantMatchGuard),
        boxed(NoRedundantPattern),
        boxed(NoReturnAssign),
        boxed(NoSelfAssign),
        boxed(NoShadowRestrictedNames),
        boxed(NoSingleElementTuple),
        boxed(NoTemplateCurlyInString),
        boxed(NoUnusedExceptRecursion),
        boxed(NoUselessBackreference),
        boxed(NoUselessComputedKey),
        boxed(NoUselessConcat),
        boxed(NoUselessConstructor),
        boxed(NoUselessEscape),
        boxed(NoUselessRename),
        boxed(NoUselessReturn),
        boxed(ReturnAwait),
        boxed(RequireAwait),
        boxed(RequireElseInIfChain),
        boxed(RequireYield),
    ]
}
