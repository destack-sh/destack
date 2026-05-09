mod array_type;
mod catch_error_name;
mod comment_casing;
mod comment_layout;
mod comment_punctuation;
mod consistent_extension_style;
mod consistent_type_definitions;
mod consistent_type_imports;
mod default_param_last;
mod dot_notation;
mod explicit_function_return_type;
mod filename_case;
mod grouped_accessor_pairs;
mod no_boolean_literal_compare;
mod no_collapsible_if;
mod no_duplicate_string;
mod no_duplicate_type_constituents;
mod no_else_return;
mod no_empty_interface;
mod no_extra_boolean_cast;
mod no_lonely_if;
mod no_negated_condition;
mod no_nested_template_literal;
mod no_nested_ternary;
mod no_redundant_type_constituents;
mod no_unnecessary_lambda;
mod no_unnecessary_template_expression;
mod no_unneeded_ternary;
mod object_shorthand;
mod operator_assignment;
mod prefer_array_filter;
mod prefer_array_find;
mod prefer_array_map;
mod prefer_array_some;
mod prefer_arrow_callback;
mod prefer_as_const;
mod prefer_const;
mod prefer_exponentiation_operator;
mod prefer_expression;
mod prefer_flat_map;
mod prefer_fragment_shorthand;
mod prefer_if_else_over_match_bool;
mod prefer_implicit_return;
mod prefer_loop;
mod prefer_match;
mod prefer_named_extension;
mod prefer_nullish_coalescing;
mod prefer_numeric_literals;
mod prefer_object_has_own;
mod prefer_object_spread;
mod prefer_pattern_over_guard;
mod prefer_precise_numeric;
mod prefer_promise_reject_errors;
mod prefer_readonly;
mod prefer_self_closing_tree;
mod prefer_set_over_empty_map;
mod prefer_string_replaceall;
mod prefer_struct;
mod prefer_struct_literal;
mod prefer_template;
mod prefer_tuple;
mod prefer_tuple_destructure;
mod prefer_tuple_swap;
mod prefer_unary_negation;
mod promise_function_async;
mod require_jsdoc;
mod require_returns_doc;
mod restrict_template_expressions;
mod sort_imports;
mod symbol_description;
mod yoda;

use crate::{BoxedLintRule, boxed};

pub use array_type::*;
pub use catch_error_name::*;
pub use comment_casing::*;
pub use comment_layout::*;
pub use comment_punctuation::*;
pub use consistent_extension_style::*;
pub use consistent_type_definitions::*;
pub use consistent_type_imports::*;
pub use default_param_last::*;
pub use dot_notation::*;
pub use explicit_function_return_type::*;
pub use filename_case::*;
pub use grouped_accessor_pairs::*;
pub use no_boolean_literal_compare::*;
pub use no_collapsible_if::*;
pub use no_duplicate_string::*;
pub use no_duplicate_type_constituents::*;
pub use no_else_return::*;
pub use no_empty_interface::*;
pub use no_extra_boolean_cast::*;
pub use no_lonely_if::*;
pub use no_negated_condition::*;
pub use no_nested_template_literal::*;
pub use no_nested_ternary::*;
pub use no_redundant_type_constituents::*;
pub use no_unnecessary_lambda::*;
pub use no_unnecessary_template_expression::*;
pub use no_unneeded_ternary::*;
pub use object_shorthand::*;
pub use operator_assignment::*;
pub use prefer_array_filter::*;
pub use prefer_array_find::*;
pub use prefer_array_map::*;
pub use prefer_array_some::*;
pub use prefer_arrow_callback::*;
pub use prefer_as_const::*;
pub use prefer_const::*;
pub use prefer_exponentiation_operator::*;
pub use prefer_expression::*;
pub use prefer_flat_map::*;
pub use prefer_fragment_shorthand::*;
pub use prefer_if_else_over_match_bool::*;
pub use prefer_implicit_return::*;
pub use prefer_loop::*;
pub use prefer_match::*;
pub use prefer_named_extension::*;
pub use prefer_nullish_coalescing::*;
pub use prefer_numeric_literals::*;
pub use prefer_object_has_own::*;
pub use prefer_object_spread::*;
pub use prefer_pattern_over_guard::*;
pub use prefer_precise_numeric::*;
pub use prefer_promise_reject_errors::*;
pub use prefer_readonly::*;
pub use prefer_self_closing_tree::*;
pub use prefer_set_over_empty_map::*;
pub use prefer_string_replaceall::*;
pub use prefer_struct::*;
pub use prefer_struct_literal::*;
pub use prefer_template::*;
pub use prefer_tuple::*;
pub use prefer_tuple_destructure::*;
pub use prefer_tuple_swap::*;
pub use prefer_unary_negation::*;
pub use promise_function_async::*;
pub use require_jsdoc::*;
pub use require_returns_doc::*;
pub use restrict_template_expressions::*;
pub use sort_imports::*;
pub use symbol_description::*;
pub use yoda::*;

/// Get all style rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(CatchErrorName),
        boxed(ArrayType),
        boxed(CommentCasing),
        boxed(CommentLayout),
        boxed(CommentPunctuation),
        boxed(ConsistentExtensionStyle),
        boxed(ConsistentTypeDefinitions),
        boxed(ConsistentTypeImports),
        boxed(DefaultParamLast),
        boxed(DotNotation),
        boxed(ExplicitFunctionReturnType),
        boxed(FilenameCaseRule),
        boxed(GroupedAccessorPairs),
        boxed(NoBooleanLiteralCompare),
        boxed(NoCollapsibleIf),
        boxed(NoDuplicateString),
        boxed(NoDuplicateTypeConstituents),
        boxed(NoElseReturn),
        boxed(NoEmptyInterface),
        boxed(NoExtraBooleanCast),
        boxed(NoLonelyIf),
        boxed(NoNegatedCondition),
        boxed(NoNestedTemplateLiteral),
        boxed(NoNestedTernary),
        boxed(NoRedundantTypeConstituents),
        boxed(NoUnnecessaryLambda),
        boxed(NoUnnecessaryTemplateExpression),
        boxed(NoUnneededTernary),
        boxed(ObjectShorthand),
        boxed(OperatorAssignment),
        boxed(PreferArrayFilter),
        boxed(PreferArrayFind),
        boxed(PreferArrayMap),
        boxed(PreferArraySome),
        boxed(PreferArrowCallback),
        boxed(PreferAsConst),
        boxed(PreferConst),
        boxed(PreferExponentiationOperator),
        boxed(PreferExpression),
        boxed(PreferFlatMap),
        boxed(PreferFragmentShorthand),
        boxed(PreferIfElseOverMatchBool),
        boxed(PreferImplicitReturn),
        boxed(PreferLoop),
        boxed(PreferMatch),
        boxed(PreferPatternOverGuard),
        boxed(PreferPromiseRejectErrors),
        boxed(PreferReadonly),
        boxed(PreferSelfClosingTree),
        boxed(PreferObjectSpread),
        boxed(PreferStruct),
        boxed(PreferStructLiteral),
        boxed(PreferSetOverEmptyMap),
        boxed(PreferStringReplaceAll),
        boxed(PreferTuple),
        boxed(PreferTupleDestructure),
        boxed(PreferTupleSwap),
        boxed(PreferNamedExtension),
        boxed(PreferNullishCoalescing),
        boxed(PreferNumericLiterals),
        boxed(PreferObjectHasOwn),
        boxed(PreferPreciseNumeric),
        boxed(PreferTemplate),
        boxed(PreferUnaryNegation),
        boxed(PromiseFunctionAsync),
        boxed(RequireJsdoc),
        boxed(RequireReturnsDoc),
        boxed(RestrictTemplateExpressions),
        boxed(SortImports),
        boxed(SymbolDescription),
        boxed(Yoda),
    ]
}
