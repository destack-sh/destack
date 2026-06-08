use serde::{Deserialize, Serialize};

use super::{ConditionAssignmentMode, EmptyFunctionKind, ReturnAwaitMode};

/// Correctness-category linter options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinterCorrectnessOptions {
    /// Ignore explicit `void` wrappers in `no-floating-promises`.
    pub no_floating_promises_ignore_void: bool,
    /// Allow explicit `void` returns in `no-promise-executor-return`.
    pub no_promise_executor_return_allow_void: bool,
    /// Check callback positions in `no-misused-promises`.
    pub no_misused_promises_check_callbacks: bool,
    /// Check conditionals in `no-misused-promises`.
    pub no_misused_promises_check_conditionals: bool,
    /// Check spread positions in `no-misused-promises`.
    pub no_misused_promises_check_spreads: bool,
    /// Check switch discriminants and cases in `use-isnan`.
    pub use_isnan_enforce_for_switch_case: bool,
    /// Check `indexOf` and `lastIndexOf` calls in `use-isnan`.
    pub use_isnan_enforce_for_index_of: bool,
    /// Check property assignments in `no-self-assign`.
    pub no_self_assign_check_properties: bool,
    /// Ignore explicit `void` wrappers in `no-confusing-void-expression`.
    pub no_confusing_void_expression_ignore_void_operator: bool,
    /// Ignore returned void expressions inside void-returning functions.
    pub no_confusing_void_expression_ignore_void_returning_functions: bool,
    /// Assignment policy for `no-cond-assign`.
    pub no_cond_assign_mode: ConditionAssignmentMode,
    /// Allow empty catch blocks in `no-empty`.
    pub no_empty_allow_empty_catch: bool,
    /// Allow empty object patterns in parameter position for `no-empty-pattern`.
    pub no_empty_pattern_allow_object_patterns_as_parameters: bool,
    /// Allowed empty function kinds in `no-empty-function`.
    pub no_empty_function_allow: Vec<EmptyFunctionKind>,
    /// Allow empty switch cases in `no-fallthrough`.
    pub no_fallthrough_allow_empty_case: bool,
    /// Regex pattern for intentional `no-fallthrough` comments.
    pub no_fallthrough_comment_pattern: Option<String>,
    /// Report unused intentional `no-fallthrough` comments.
    pub no_fallthrough_report_unused_comment: bool,
    /// Await policy for the `return-await` rule.
    pub return_await_mode: ReturnAwaitMode,
    /// Parameter name prefixes ignored by `no-unused-parameters`.
    pub ignored_unused_parameter_prefixes: Vec<String>,
    /// Ignore destructuring aliases in `no-useless-rename`.
    pub no_useless_rename_ignore_destructuring: bool,
    /// Ignore import aliases in `no-useless-rename`.
    pub no_useless_rename_ignore_import: bool,
    /// Ignore export aliases in `no-useless-rename`.
    pub no_useless_rename_ignore_export: bool,
    /// Regex characters allowed by `no-useless-escape`.
    pub no_useless_escape_allow_regex_characters: Vec<String>,
}

impl Default for LinterCorrectnessOptions {
    fn default() -> Self {
        Self {
            no_floating_promises_ignore_void: true,
            no_promise_executor_return_allow_void: false,
            no_misused_promises_check_callbacks: true,
            no_misused_promises_check_conditionals: true,
            no_misused_promises_check_spreads: true,
            use_isnan_enforce_for_switch_case: true,
            use_isnan_enforce_for_index_of: false,
            no_self_assign_check_properties: true,
            no_confusing_void_expression_ignore_void_operator: false,
            no_confusing_void_expression_ignore_void_returning_functions: false,
            no_cond_assign_mode: ConditionAssignmentMode::default(),
            no_empty_allow_empty_catch: false,
            no_empty_pattern_allow_object_patterns_as_parameters: false,
            no_empty_function_allow: Vec::new(),
            no_fallthrough_allow_empty_case: false,
            no_fallthrough_comment_pattern: None,
            no_fallthrough_report_unused_comment: false,
            return_await_mode: ReturnAwaitMode::default(),
            ignored_unused_parameter_prefixes: vec!["_".to_string()],
            no_useless_rename_ignore_destructuring: false,
            no_useless_rename_ignore_import: false,
            no_useless_rename_ignore_export: false,
            no_useless_escape_allow_regex_characters: Vec::new(),
        }
    }
}
