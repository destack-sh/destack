mod catch_error_name;
mod comment_casing;
mod comment_layout;
mod comment_punctuation;
mod consistent_extension_style;
mod consistent_type_definitions;
mod consistent_type_imports;
mod default_param_last;
mod dot_notation;
mod eqeqeq;
mod explicit_function_return_type;
mod filename_case;
mod grouped_accessor_pairs;
mod no_boolean_literal_compare;
mod no_collapsible_if;
mod no_duplicate_string;
mod no_else_return;
mod no_empty_interface;
mod no_lonely_if;
mod no_negated_condition;
mod no_nested_template_literal;
mod no_nested_ternary;
mod no_unnecessary_lambda;
mod no_unneeded_ternary;
mod no_var;
mod object_shorthand;
mod operator_assignment;
mod prefer_arrow_callback;
mod prefer_as_const;
mod prefer_expression;
mod prefer_fragment_shorthand;
mod prefer_if_else_over_match_bool;
mod prefer_implicit_return;
mod prefer_inclusive_range;
mod prefer_loop;
mod prefer_named_extension;
mod prefer_pattern_over_guard;
mod prefer_precise_numeric;
mod prefer_range_contains;
mod prefer_range_literal;
mod prefer_self_closing_tree;
mod prefer_struct;
mod prefer_struct_literal;
mod prefer_template;
mod prefer_tuple;
mod prefer_tuple_destructure;
mod prefer_tuple_swap;
mod prefer_unary_negation;
mod require_jsdoc;
mod require_returns_doc;
mod sort_imports;
mod yoda;

use crate::{BoxedLintRule, boxed};

pub use catch_error_name::*;
pub use comment_casing::*;
pub use comment_layout::*;
pub use comment_punctuation::*;
pub use consistent_extension_style::*;
pub use consistent_type_definitions::*;
pub use consistent_type_imports::*;
pub use default_param_last::*;
pub use dot_notation::*;
pub use eqeqeq::*;
pub use explicit_function_return_type::*;
pub use filename_case::*;
pub use grouped_accessor_pairs::*;
pub use no_boolean_literal_compare::*;
pub use no_collapsible_if::*;
pub use no_duplicate_string::*;
pub use no_else_return::*;
pub use no_empty_interface::*;
pub use no_lonely_if::*;
pub use no_negated_condition::*;
pub use no_nested_template_literal::*;
pub use no_nested_ternary::*;
pub use no_unnecessary_lambda::*;
pub use no_unneeded_ternary::*;
pub use no_var::*;
pub use object_shorthand::*;
pub use operator_assignment::*;
pub use prefer_arrow_callback::*;
pub use prefer_as_const::*;
pub use prefer_expression::*;
pub use prefer_fragment_shorthand::*;
pub use prefer_if_else_over_match_bool::*;
pub use prefer_implicit_return::*;
pub use prefer_inclusive_range::*;
pub use prefer_loop::*;
pub use prefer_named_extension::*;
pub use prefer_pattern_over_guard::*;
pub use prefer_precise_numeric::*;
pub use prefer_range_contains::*;
pub use prefer_range_literal::*;
pub use prefer_self_closing_tree::*;
pub use prefer_struct::*;
pub use prefer_struct_literal::*;
pub use prefer_template::*;
pub use prefer_tuple::*;
pub use prefer_tuple_destructure::*;
pub use prefer_tuple_swap::*;
pub use prefer_unary_negation::*;
pub use require_jsdoc::*;
pub use require_returns_doc::*;
pub use sort_imports::*;
pub use yoda::*;

/// Get all style rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(CatchErrorName),
        boxed(CommentCasing),
        boxed(CommentLayout),
        boxed(CommentPunctuation),
        boxed(ConsistentExtensionStyle),
        boxed(ConsistentTypeDefinitions),
        boxed(ConsistentTypeImports),
        boxed(DefaultParamLast),
        boxed(DotNotation),
        boxed(Eqeqeq),
        boxed(ExplicitFunctionReturnType),
        boxed(FilenameCaseRule),
        boxed(GroupedAccessorPairs),
        boxed(NoBooleanLiteralCompare),
        boxed(NoCollapsibleIf),
        boxed(NoDuplicateString),
        boxed(NoElseReturn),
        boxed(NoEmptyInterface),
        boxed(NoLonelyIf),
        boxed(NoNegatedCondition),
        boxed(NoNestedTemplateLiteral),
        boxed(NoNestedTernary),
        boxed(NoUnnecessaryLambda),
        boxed(NoUnneededTernary),
        boxed(NoVar),
        boxed(ObjectShorthand),
        boxed(OperatorAssignment),
        boxed(PreferArrowCallback),
        boxed(PreferAsConst),
        boxed(PreferExpression),
        boxed(PreferFragmentShorthand),
        boxed(PreferIfElseOverMatchBool),
        boxed(PreferImplicitReturn),
        boxed(PreferInclusiveRange),
        boxed(PreferLoop),
        boxed(PreferPatternOverGuard),
        boxed(PreferRangeContains),
        boxed(PreferSelfClosingTree),
        boxed(PreferStruct),
        boxed(PreferStructLiteral),
        boxed(PreferTuple),
        boxed(PreferTupleDestructure),
        boxed(PreferTupleSwap),
        boxed(PreferNamedExtension),
        boxed(PreferPreciseNumeric),
        boxed(PreferRangeLiteral),
        boxed(PreferTemplate),
        boxed(PreferUnaryNegation),
        boxed(RequireJsdoc),
        boxed(RequireReturnsDoc),
        boxed(SortImports),
        boxed(Yoda),
    ]
}
