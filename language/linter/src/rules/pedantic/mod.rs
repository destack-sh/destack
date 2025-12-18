use crate::BoxedLintRule;

mod comment_casing;
mod comment_layout;
mod comment_punctuation;
mod explicit_function_return_type;
mod guard_for_in;
mod no_class_for_data;
mod no_useless_underscore_binding;
mod prefer_expression_over_let_if;
mod prefer_if_let;
mod prefer_named_extension;
mod prefer_precise_numeric;
mod prefer_simplified_comparison;
mod require_await;
mod require_else_in_if_chain;
mod require_jsdoc;
mod require_returns_doc;

pub use comment_casing::*;
pub use comment_layout::*;
pub use comment_punctuation::*;
pub use explicit_function_return_type::*;
pub use guard_for_in::*;
pub use no_class_for_data::*;
pub use no_useless_underscore_binding::*;
pub use prefer_expression_over_let_if::*;
pub use prefer_if_let::*;
pub use prefer_named_extension::*;
pub use prefer_precise_numeric::*;
pub use prefer_simplified_comparison::*;
pub use require_await::*;
pub use require_else_in_if_chain::*;
pub use require_jsdoc::*;
pub use require_returns_doc::*;

/// Get all pedantic rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        Box::new(CommentCasing),
        Box::new(CommentLayout),
        Box::new(CommentPunctuation),
        Box::new(ExplicitFunctionReturnType),
        Box::new(GuardForIn),
        Box::new(NoClassForData),
        Box::new(NoUselessUnderscoreBinding),
        Box::new(PreferExpressionOverLetIf),
        Box::new(PreferIfLet),
        Box::new(PreferNamedExtension),
        Box::new(PreferPreciseNumeric),
        Box::new(PreferSimplifiedComparison),
        Box::new(RequireAwait),
        Box::new(RequireElseInIfChain),
        Box::new(RequireJsdoc),
        Box::new(RequireReturnsDoc),
    ]
}
