use serde::Deserialize;

use super::{
    CyclomaticComplexityVariant, CyclomaticComplexityVariantJson, LinterOptions,
    MaxParamsCountThis, MaxParamsCountThisJson,
};

/// Complexity-category linter options.
#[derive(Debug, Clone)]
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

/// Complexity-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterComplexityJson {
    /// Maximum boolean parameters or fields.
    pub max_booleans: Option<usize>,
    /// Maximum branching factor.
    pub max_branching_factor: Option<usize>,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: Option<usize>,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: Option<usize>,
    /// Switch counting variant for `cyclomatic-complexity`.
    pub cyclomatic_complexity_variant: Option<CyclomaticComplexityVariantJson>,
    /// Maximum nesting depth.
    pub max_depth: Option<usize>,
    /// Maximum generic parameters.
    pub max_generic_params: Option<usize>,
    /// Maximum lines per file.
    pub max_lines: Option<usize>,
    /// Ignore full-line comments in `max-lines`.
    pub max_lines_skip_comments: Option<bool>,
    /// Ignore blank lines in `max-lines`.
    pub max_lines_skip_blank_lines: Option<bool>,
    /// Maximum lines per function.
    pub max_lines_per_function: Option<usize>,
    /// Ignore full-line comments in `max-lines-per-function`.
    pub max_lines_per_function_skip_comments: Option<bool>,
    /// Ignore blank lines in `max-lines-per-function`.
    pub max_lines_per_function_skip_blank_lines: Option<bool>,
    /// Count immediately invoked functions in `max-lines-per-function`.
    pub max_lines_per_function_iifes: Option<bool>,
    /// Maximum callback nesting.
    pub max_nested_callbacks: Option<usize>,
    /// Maximum function parameters.
    pub max_params: Option<usize>,
    /// Count `this` parameters in `max-params`.
    pub max_params_count_this: Option<MaxParamsCountThisJson>,
    /// Maximum statements per function.
    pub max_statements: Option<usize>,
    /// Ignore top-level functions in `max-statements`.
    pub max_statements_ignore_top_level_functions: Option<bool>,
    /// Maximum return statements per function.
    pub max_return_statements: Option<usize>,
    /// Maximum switch cases per switch statement.
    pub max_switch_cases: Option<usize>,
    /// Maximum variants in a union type or enum.
    pub max_type_variants: Option<usize>,
    /// Maximum fields in a struct, class, or interface.
    pub max_type_fields: Option<usize>,
    /// Maximum type complexity.
    pub max_type_complexity: Option<usize>,
    /// Maximum duplicate string occurrences.
    pub max_duplicate_string_occurrences: Option<usize>,
    /// Maximum statements in a try block.
    pub max_try_block_statements: Option<usize>,
    /// Ignore non-declaration chains in `no-multi-assign`.
    pub no_multi_assign_ignore_non_declaration: Option<bool>,
    /// Allow short-circuit expressions in `no-unused-expressions`.
    pub no_unused_expressions_allow_short_circuit: Option<bool>,
    /// Allow ternary expressions in `no-unused-expressions`.
    pub no_unused_expressions_allow_ternary: Option<bool>,
    /// Allow tagged templates in `no-unused-expressions`.
    pub no_unused_expressions_allow_tagged_templates: Option<bool>,
    /// Enforce JSX-like tree expressions in `no-unused-expressions`.
    pub no_unused_expressions_enforce_for_jsx: Option<bool>,
    /// Ignore directive prologues in `no-unused-expressions`.
    pub no_unused_expressions_ignore_directives: Option<bool>,
}

impl LinterComplexityJson {
    /// Validate complexity-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Apply complexity-category options to one linter options struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(max_booleans) = self.max_booleans {
            options.complexity.max_booleans = max_booleans;
        }

        if let Some(max_branching_factor) = self.max_branching_factor {
            options.complexity.max_branching_factor = max_branching_factor;
        }

        if let Some(max_cognitive_complexity) = self.max_cognitive_complexity {
            options.complexity.max_cognitive_complexity = max_cognitive_complexity;
        }

        if let Some(max_cyclomatic_complexity) = self.max_cyclomatic_complexity {
            options.complexity.max_cyclomatic_complexity = max_cyclomatic_complexity;
        }

        if let Some(cyclomatic_complexity_variant) = self.cyclomatic_complexity_variant {
            options.complexity.cyclomatic_complexity_variant = cyclomatic_complexity_variant.into();
        }

        if let Some(max_depth) = self.max_depth {
            options.complexity.max_depth = max_depth;
        }

        if let Some(max_generic_params) = self.max_generic_params {
            options.complexity.max_generic_params = max_generic_params;
        }

        if let Some(max_lines) = self.max_lines {
            options.complexity.max_lines = max_lines;
        }

        if let Some(max_lines_skip_comments) = self.max_lines_skip_comments {
            options.complexity.max_lines_skip_comments = max_lines_skip_comments;
        }

        if let Some(max_lines_skip_blank_lines) = self.max_lines_skip_blank_lines {
            options.complexity.max_lines_skip_blank_lines = max_lines_skip_blank_lines;
        }

        if let Some(max_lines_per_function) = self.max_lines_per_function {
            options.complexity.max_lines_per_function = max_lines_per_function;
        }

        if let Some(max_lines_per_function_skip_comments) =
            self.max_lines_per_function_skip_comments
        {
            options.complexity.max_lines_per_function_skip_comments =
                max_lines_per_function_skip_comments;
        }

        if let Some(max_lines_per_function_skip_blank_lines) =
            self.max_lines_per_function_skip_blank_lines
        {
            options.complexity.max_lines_per_function_skip_blank_lines =
                max_lines_per_function_skip_blank_lines;
        }

        if let Some(max_lines_per_function_iifes) = self.max_lines_per_function_iifes {
            options.complexity.max_lines_per_function_iifes = max_lines_per_function_iifes;
        }

        if let Some(max_nested_callbacks) = self.max_nested_callbacks {
            options.complexity.max_nested_callbacks = max_nested_callbacks;
        }

        if let Some(max_params) = self.max_params {
            options.complexity.max_params = max_params;
        }

        if let Some(max_params_count_this) = self.max_params_count_this {
            options.complexity.max_params_count_this = max_params_count_this.into();
        }

        if let Some(max_statements) = self.max_statements {
            options.complexity.max_statements = max_statements;
        }

        if let Some(max_statements_ignore_top_level_functions) =
            self.max_statements_ignore_top_level_functions
        {
            options.complexity.max_statements_ignore_top_level_functions =
                max_statements_ignore_top_level_functions;
        }

        if let Some(max_return_statements) = self.max_return_statements {
            options.complexity.max_return_statements = max_return_statements;
        }

        if let Some(max_switch_cases) = self.max_switch_cases {
            options.complexity.max_switch_cases = max_switch_cases;
        }

        if let Some(max_type_variants) = self.max_type_variants {
            options.complexity.max_type_variants = max_type_variants;
        }

        if let Some(max_type_fields) = self.max_type_fields {
            options.complexity.max_type_fields = max_type_fields;
        }

        if let Some(max_type_complexity) = self.max_type_complexity {
            options.complexity.max_type_complexity = max_type_complexity;
        }

        if let Some(max_duplicate_string_occurrences) = self.max_duplicate_string_occurrences {
            options.complexity.max_duplicate_string_occurrences = max_duplicate_string_occurrences;
        }

        if let Some(max_try_block_statements) = self.max_try_block_statements {
            options.complexity.max_try_block_statements = max_try_block_statements;
        }

        if let Some(no_multi_assign_ignore_non_declaration) =
            self.no_multi_assign_ignore_non_declaration
        {
            options.complexity.no_multi_assign_ignore_non_declaration =
                no_multi_assign_ignore_non_declaration;
        }

        if let Some(no_unused_expressions_allow_short_circuit) =
            self.no_unused_expressions_allow_short_circuit
        {
            options.complexity.no_unused_expressions_allow_short_circuit =
                no_unused_expressions_allow_short_circuit;
        }

        if let Some(no_unused_expressions_allow_ternary) = self.no_unused_expressions_allow_ternary
        {
            options.complexity.no_unused_expressions_allow_ternary =
                no_unused_expressions_allow_ternary;
        }

        if let Some(no_unused_expressions_allow_tagged_templates) =
            self.no_unused_expressions_allow_tagged_templates
        {
            options
                .complexity
                .no_unused_expressions_allow_tagged_templates =
                no_unused_expressions_allow_tagged_templates;
        }

        if let Some(no_unused_expressions_enforce_for_jsx) =
            self.no_unused_expressions_enforce_for_jsx
        {
            options.complexity.no_unused_expressions_enforce_for_jsx =
                no_unused_expressions_enforce_for_jsx;
        }

        if let Some(no_unused_expressions_ignore_directives) =
            self.no_unused_expressions_ignore_directives
        {
            options.complexity.no_unused_expressions_ignore_directives =
                no_unused_expressions_ignore_directives;
        }
    }
}
