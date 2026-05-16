use serde::{Deserialize, Serialize};

use super::{CyclomaticComplexityVariant, MaxParamsCountThis};

/// Complexity-category linter options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinterComplexityOptions {
    /// Maximum boolean parameters or fields.
    pub max_booleans: usize,
    /// Maximum branches in a single conditional (if/match).
    pub max_branching_factor: usize,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: usize,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: usize,
    /// Switch counting variant for `cyclomatic-complexity`.
    pub cyclomatic_complexity_variant: CyclomaticComplexityVariant,
    /// Maximum nesting depth.
    pub max_depth: usize,
    /// Maximum generic parameters (generics including const values).
    pub max_generic_params: usize,
    /// Maximum lines per file.
    pub max_lines: usize,
    /// Ignore full-line comments in `max-lines`.
    pub max_lines_skip_comments: bool,
    /// Ignore blank lines in `max-lines`.
    pub max_lines_skip_blank_lines: bool,
    /// Maximum lines per function.
    pub max_lines_per_function: usize,
    /// Ignore full-line comments in `max-lines-per-function`.
    pub max_lines_per_function_skip_comments: bool,
    /// Ignore blank lines in `max-lines-per-function`.
    pub max_lines_per_function_skip_blank_lines: bool,
    /// Count immediately invoked functions in `max-lines-per-function`.
    pub max_lines_per_function_iifes: bool,
    /// Maximum callback nesting.
    pub max_nested_callbacks: usize,
    /// Maximum function parameters.
    pub max_params: usize,
    /// Count `this` parameters in `max-params`.
    pub max_params_count_this: MaxParamsCountThis,
    /// Maximum statements per function.
    pub max_statements: usize,
    /// Ignore top-level functions in `max-statements`.
    pub max_statements_ignore_top_level_functions: bool,
    /// Maximum return statements per function.
    pub max_return_statements: usize,
    /// Maximum switch cases per switch statement.
    pub max_switch_cases: usize,
    /// Maximum variants in a union type or enum.
    pub max_type_variants: usize,
    /// Maximum fields in a struct, class, or interface.
    pub max_type_fields: usize,
    /// Maximum type complexity.
    pub max_type_complexity: usize,
    /// Maximum occurrences of the same string literal before warning.
    pub max_duplicate_string_occurrences: usize,
    /// Minimum lines required to consider a block for duplicate code checks.
    pub min_duplicate_code_lines: usize,
    /// Minimum tokens required to consider a block for duplicate code checks.
    pub min_duplicate_code_tokens: usize,
    /// Minimum similarity percent for near duplicate code matching.
    pub min_duplicate_code_near_similarity: u8,
    /// Maximum statements in a try block.
    pub max_try_block_statements: usize,
    /// Ignore non-declaration chains in `no-multi-assign`.
    pub no_multi_assign_ignore_non_declaration: bool,
    /// Allow short-circuit expressions in `no-unused-expressions`.
    pub no_unused_expressions_allow_short_circuit: bool,
    /// Allow ternary expressions in `no-unused-expressions`.
    pub no_unused_expressions_allow_ternary: bool,
    /// Allow tagged templates in `no-unused-expressions`.
    pub no_unused_expressions_allow_tagged_templates: bool,
    /// Enforce JSX-like tree expressions in `no-unused-expressions`.
    pub no_unused_expressions_enforce_for_jsx: bool,
    /// Ignore directive prologues in `no-unused-expressions`.
    pub no_unused_expressions_ignore_directives: bool,
}

impl Default for LinterComplexityOptions {
    fn default() -> Self {
        Self {
            max_booleans: 3,
            max_branching_factor: 10,
            max_cognitive_complexity: 30,
            max_cyclomatic_complexity: 40,
            cyclomatic_complexity_variant: CyclomaticComplexityVariant::default(),
            max_depth: 4,
            max_generic_params: 4,
            max_lines: 500,
            max_lines_skip_comments: false,
            max_lines_skip_blank_lines: false,
            max_lines_per_function: 50,
            max_lines_per_function_skip_comments: false,
            max_lines_per_function_skip_blank_lines: false,
            max_lines_per_function_iifes: false,
            max_nested_callbacks: 4,
            max_params: 4,
            max_params_count_this: MaxParamsCountThis::default(),
            max_statements: 50,
            max_statements_ignore_top_level_functions: false,
            max_return_statements: 10,
            max_switch_cases: 20,
            max_type_variants: 20,
            max_type_fields: 30,
            max_type_complexity: 10,
            max_duplicate_string_occurrences: 6,
            min_duplicate_code_lines: 6,
            min_duplicate_code_tokens: 32,
            min_duplicate_code_near_similarity: 100,
            max_try_block_statements: 20,
            no_multi_assign_ignore_non_declaration: false,
            no_unused_expressions_allow_short_circuit: false,
            no_unused_expressions_allow_ternary: false,
            no_unused_expressions_allow_tagged_templates: false,
            no_unused_expressions_enforce_for_jsx: false,
            no_unused_expressions_ignore_directives: false,
        }
    }
}
