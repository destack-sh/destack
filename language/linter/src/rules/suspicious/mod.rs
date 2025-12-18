mod guard_for_in;
mod no_cond_assign;
mod no_confusing_assignment;
mod no_confusing_non_null_assertion;
mod no_constant_assertion;
mod no_constructor_return;
mod no_debugger;
mod no_dupe_else_if;
mod no_duplicate_match_arms;
mod no_empty;
mod no_empty_function;
mod no_empty_pattern;
mod no_empty_static_block;
mod no_extra_non_null_assertion;
mod no_incomplete_range;
mod no_inner_declarations;
mod no_misleading_character_class;
mod no_negation_in_equality_check;
mod no_redundant_pattern;
mod no_return_assign;
mod no_self_assign;
mod no_single_element_tuple;
mod no_template_curly_in_string;
mod no_useless_backreference;
mod no_useless_catch;
mod no_useless_computed_key;
mod no_useless_concat;
mod no_useless_constructor;
mod no_useless_escape;
mod no_useless_rename;
mod no_useless_return;
mod prefer_match;
mod require_await;
mod require_else_in_if_chain;
mod require_yield;

use crate::{BoxedLintRule, boxed};

pub use guard_for_in::*;
pub use no_cond_assign::*;
pub use no_confusing_assignment::*;
pub use no_confusing_non_null_assertion::*;
pub use no_constant_assertion::*;
pub use no_constructor_return::*;
pub use no_debugger::*;
pub use no_dupe_else_if::*;
pub use no_duplicate_match_arms::*;
pub use no_empty::*;
pub use no_empty_function::*;
pub use no_empty_pattern::*;
pub use no_empty_static_block::*;
pub use no_extra_non_null_assertion::*;
pub use no_incomplete_range::*;
pub use no_inner_declarations::*;
pub use no_misleading_character_class::*;
pub use no_negation_in_equality_check::*;
pub use no_redundant_pattern::*;
pub use no_return_assign::*;
pub use no_self_assign::*;
pub use no_single_element_tuple::*;
pub use no_template_curly_in_string::*;
pub use no_useless_backreference::*;
pub use no_useless_catch::*;
pub use no_useless_computed_key::*;
pub use no_useless_concat::*;
pub use no_useless_constructor::*;
pub use no_useless_escape::*;
pub use no_useless_rename::*;
pub use no_useless_return::*;
pub use prefer_match::*;
pub use require_await::*;
pub use require_else_in_if_chain::*;
pub use require_yield::*;

/// Get all suspicious rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(GuardForIn),
        boxed(NoCondAssign),
        boxed(NoConfusingAssignment),
        boxed(NoConstantAssertion),
        boxed(NoConfusingNonNullAssertion),
        boxed(NoConstructorReturn),
        boxed(NoDebugger),
        boxed(NoDupeElseIf),
        boxed(NoDuplicateMatchArms),
        boxed(NoEmpty),
        boxed(NoEmptyFunction),
        boxed(NoEmptyPattern),
        boxed(NoEmptyStaticBlock),
        boxed(NoExtraNonNullAssertion),
        boxed(NoIncompleteRange),
        boxed(NoInnerDeclarations),
        boxed(NoMisleadingCharacterClass),
        boxed(NoNegationInEqualityCheck),
        boxed(NoRedundantPattern),
        boxed(NoReturnAssign),
        boxed(NoSelfAssign),
        boxed(NoSingleElementTuple),
        boxed(NoTemplateCurlyInString),
        boxed(NoUselessBackreference),
        boxed(NoUselessCatch),
        boxed(NoUselessComputedKey),
        boxed(NoUselessConcat),
        boxed(NoUselessConstructor),
        boxed(NoUselessEscape),
        boxed(NoUselessRename),
        boxed(NoUselessReturn),
        boxed(PreferMatch),
        boxed(RequireAwait),
        boxed(RequireElseInIfChain),
        boxed(RequireYield),
    ]
}
