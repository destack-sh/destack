mod cognitive_complexity;
mod cyclomatic_complexity;
mod max_branching_factor;
mod max_depth;
mod max_generic_params;
mod max_lines;
mod max_lines_per_function;
mod max_nested_callbacks;
mod max_params;
mod max_return_statements;
mod max_statements;
mod max_switch_cases;
mod max_type_fields;
mod max_type_variants;
mod no_complex_boolean_expression;
mod no_complex_type;
mod no_duplicate_code;
mod no_excessive_booleans;
mod no_multi_assign;
mod no_multi_declarators;
mod no_nested_switch;
mod no_unused_expressions;
mod no_useless_underscore_binding;
mod prefer_expression_over_let_if;
mod prefer_if_let;
mod prefer_simplified_comparison;

use crate::{BoxedLintRule, boxed};

pub use cognitive_complexity::*;
pub use cyclomatic_complexity::*;
pub use max_branching_factor::*;
pub use max_depth::*;
pub use max_generic_params::*;
pub use max_lines::*;
pub use max_lines_per_function::*;
pub use max_nested_callbacks::*;
pub use max_params::*;
pub use max_return_statements::*;
pub use max_statements::*;
pub use max_switch_cases::*;
pub use max_type_fields::*;
pub use max_type_variants::*;
pub use no_complex_boolean_expression::*;
pub use no_complex_type::*;
pub use no_duplicate_code::*;
pub use no_excessive_booleans::*;
pub use no_multi_assign::*;
pub use no_multi_declarators::*;
pub use no_nested_switch::*;
pub use no_unused_expressions::*;
pub use no_useless_underscore_binding::*;
pub use prefer_expression_over_let_if::*;
pub use prefer_if_let::*;
pub use prefer_simplified_comparison::*;

/// Get all complexity rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(CognitiveComplexity),
        boxed(CyclomaticComplexity),
        boxed(MaxBranchingFactor),
        boxed(MaxDepth),
        boxed(MaxLines),
        boxed(MaxLinesPerFunction),
        boxed(MaxNestedCallbacks),
        boxed(MaxParams),
        boxed(MaxReturnStatements),
        boxed(MaxStatements),
        boxed(MaxGenericParams),
        boxed(MaxSwitchCases),
        boxed(MaxTypeFields),
        boxed(MaxTypeVariants),
        boxed(NoComplexBooleanExpression),
        boxed(NoComplexType),
        boxed(NoDuplicateCode),
        boxed(NoExcessiveBooleans),
        boxed(NoNestedSwitch),
        boxed(NoMultiAssign),
        boxed(NoMultiDeclarators),
        boxed(NoUnusedExpressions),
        boxed(NoUselessUnderscoreBinding),
        boxed(PreferExpressionOverLetIf),
        boxed(PreferIfLet),
        boxed(PreferSimplifiedComparison),
    ]
}
