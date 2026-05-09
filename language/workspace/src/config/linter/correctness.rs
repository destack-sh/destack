use serde::Deserialize;

use super::{
    ConditionAssignmentMode, ConditionAssignmentModeJson, EmptyFunctionKind, EmptyFunctionKindJson,
    LinterOptions, ReturnAwaitMode, ReturnAwaitModeJson, validate_regex_patterns,
    validate_single_character_strings,
};

/// Correctness-category linter options.
#[derive(Debug, Clone)]
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

/// Correctness-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterCorrectnessJson {
    /// Await policy for the `return-await` rule.
    pub return_await_mode: Option<ReturnAwaitModeJson>,
    /// Parameter name prefixes ignored by `no-unused-parameters`.
    pub ignored_unused_parameter_prefixes: Option<Vec<String>>,
    /// Ignore destructuring aliases in `no-useless-rename`.
    pub no_useless_rename_ignore_destructuring: Option<bool>,
    /// Ignore import aliases in `no-useless-rename`.
    pub no_useless_rename_ignore_import: Option<bool>,
    /// Ignore export aliases in `no-useless-rename`.
    pub no_useless_rename_ignore_export: Option<bool>,
    /// Regex characters allowed by `no-useless-escape`.
    pub no_useless_escape_allow_regex_characters: Option<Vec<String>>,
    /// Ignore explicit `void` wrappers in `no-confusing-void-expression`.
    pub no_confusing_void_expression_ignore_void_operator: Option<bool>,
    /// Ignore returned void expressions inside void-returning functions.
    pub no_confusing_void_expression_ignore_void_returning_functions: Option<bool>,
    /// Assignment policy for `no-cond-assign`.
    pub no_cond_assign_mode: Option<ConditionAssignmentModeJson>,
    /// Allow empty catch blocks in `no-empty`.
    pub no_empty_allow_empty_catch: Option<bool>,
    /// Allow empty object patterns in parameter position for `no-empty-pattern`.
    pub no_empty_pattern_allow_object_patterns_as_parameters: Option<bool>,
    /// Allowed empty function kinds in `no-empty-function`.
    pub no_empty_function_allow: Option<Vec<EmptyFunctionKindJson>>,
    /// Allow empty switch cases in `no-fallthrough`.
    pub no_fallthrough_allow_empty_case: Option<bool>,
    /// Regex pattern for intentional `no-fallthrough` comments.
    pub no_fallthrough_comment_pattern: Option<String>,
    /// Report unused intentional `no-fallthrough` comments.
    pub no_fallthrough_report_unused_comment: Option<bool>,
    /// Ignore explicit `void` wrappers in `no-floating-promises`.
    pub no_floating_promises_ignore_void: Option<bool>,
    /// Allow explicit `void` returns in `no-promise-executor-return`.
    pub no_promise_executor_return_allow_void: Option<bool>,
    /// Check callback positions in `no-misused-promises`.
    pub no_misused_promises_check_callbacks: Option<bool>,
    /// Check conditionals in `no-misused-promises`.
    pub no_misused_promises_check_conditionals: Option<bool>,
    /// Check spread positions in `no-misused-promises`.
    pub no_misused_promises_check_spreads: Option<bool>,
    /// Check switch discriminants and cases in `use-isnan`.
    pub use_isnan_enforce_for_switch_case: Option<bool>,
    /// Check `indexOf` and `lastIndexOf` calls in `use-isnan`.
    pub use_isnan_enforce_for_index_of: Option<bool>,
    /// Check property assignments in `no-self-assign`.
    pub no_self_assign_check_properties: Option<bool>,
}

impl LinterCorrectnessJson {
    /// Validate correctness-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        validate_regex_patterns(
            "linter.noFallthroughCommentPattern",
            self.no_fallthrough_comment_pattern
                .as_ref()
                .map(std::slice::from_ref),
        )?;
        validate_single_character_strings(
            "linter.noUselessEscapeAllowRegexCharacters",
            self.no_useless_escape_allow_regex_characters.as_deref(),
        )?;

        Ok(())
    }

    /// Apply correctness-category options to one linter options struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(return_await_mode) = self.return_await_mode {
            options.correctness.return_await_mode = return_await_mode.into();
        }

        if let Some(ref ignored_unused_parameter_prefixes) = self.ignored_unused_parameter_prefixes
        {
            options.correctness.ignored_unused_parameter_prefixes =
                ignored_unused_parameter_prefixes.clone();
        }

        if let Some(no_useless_rename_ignore_destructuring) =
            self.no_useless_rename_ignore_destructuring
        {
            options.correctness.no_useless_rename_ignore_destructuring =
                no_useless_rename_ignore_destructuring;
        }

        if let Some(no_useless_rename_ignore_import) = self.no_useless_rename_ignore_import {
            options.correctness.no_useless_rename_ignore_import = no_useless_rename_ignore_import;
        }

        if let Some(no_useless_rename_ignore_export) = self.no_useless_rename_ignore_export {
            options.correctness.no_useless_rename_ignore_export = no_useless_rename_ignore_export;
        }

        if let Some(ref no_useless_escape_allow_regex_characters) =
            self.no_useless_escape_allow_regex_characters
        {
            options.correctness.no_useless_escape_allow_regex_characters =
                no_useless_escape_allow_regex_characters.clone();
        }

        if let Some(no_confusing_void_expression_ignore_void_operator) =
            self.no_confusing_void_expression_ignore_void_operator
        {
            options
                .correctness
                .no_confusing_void_expression_ignore_void_operator =
                no_confusing_void_expression_ignore_void_operator;
        }

        if let Some(no_confusing_void_expression_ignore_void_returning_functions) =
            self.no_confusing_void_expression_ignore_void_returning_functions
        {
            options
                .correctness
                .no_confusing_void_expression_ignore_void_returning_functions =
                no_confusing_void_expression_ignore_void_returning_functions;
        }

        if let Some(no_cond_assign_mode) = self.no_cond_assign_mode {
            options.correctness.no_cond_assign_mode = no_cond_assign_mode.into();
        }

        if let Some(no_empty_allow_empty_catch) = self.no_empty_allow_empty_catch {
            options.correctness.no_empty_allow_empty_catch = no_empty_allow_empty_catch;
        }

        if let Some(no_empty_pattern_allow_object_patterns_as_parameters) =
            self.no_empty_pattern_allow_object_patterns_as_parameters
        {
            options
                .correctness
                .no_empty_pattern_allow_object_patterns_as_parameters =
                no_empty_pattern_allow_object_patterns_as_parameters;
        }

        if let Some(ref no_empty_function_allow) = self.no_empty_function_allow {
            options.correctness.no_empty_function_allow = no_empty_function_allow
                .iter()
                .copied()
                .map(Into::into)
                .collect();
        }

        if let Some(no_fallthrough_allow_empty_case) = self.no_fallthrough_allow_empty_case {
            options.correctness.no_fallthrough_allow_empty_case = no_fallthrough_allow_empty_case;
        }

        if let Some(ref no_fallthrough_comment_pattern) = self.no_fallthrough_comment_pattern {
            options.correctness.no_fallthrough_comment_pattern =
                Some(no_fallthrough_comment_pattern.clone());
        }

        if let Some(no_fallthrough_report_unused_comment) =
            self.no_fallthrough_report_unused_comment
        {
            options.correctness.no_fallthrough_report_unused_comment =
                no_fallthrough_report_unused_comment;
        }

        if let Some(no_floating_promises_ignore_void) = self.no_floating_promises_ignore_void {
            options.correctness.no_floating_promises_ignore_void = no_floating_promises_ignore_void;
        }

        if let Some(no_promise_executor_return_allow_void) =
            self.no_promise_executor_return_allow_void
        {
            options.correctness.no_promise_executor_return_allow_void =
                no_promise_executor_return_allow_void;
        }

        if let Some(no_misused_promises_check_callbacks) = self.no_misused_promises_check_callbacks
        {
            options.correctness.no_misused_promises_check_callbacks =
                no_misused_promises_check_callbacks;
        }

        if let Some(no_misused_promises_check_conditionals) =
            self.no_misused_promises_check_conditionals
        {
            options.correctness.no_misused_promises_check_conditionals =
                no_misused_promises_check_conditionals;
        }

        if let Some(no_misused_promises_check_spreads) = self.no_misused_promises_check_spreads {
            options.correctness.no_misused_promises_check_spreads =
                no_misused_promises_check_spreads;
        }

        if let Some(use_isnan_enforce_for_switch_case) = self.use_isnan_enforce_for_switch_case {
            options.correctness.use_isnan_enforce_for_switch_case =
                use_isnan_enforce_for_switch_case;
        }

        if let Some(use_isnan_enforce_for_index_of) = self.use_isnan_enforce_for_index_of {
            options.correctness.use_isnan_enforce_for_index_of = use_isnan_enforce_for_index_of;
        }

        if let Some(no_self_assign_check_properties) = self.no_self_assign_check_properties {
            options.correctness.no_self_assign_check_properties = no_self_assign_check_properties;
        }
    }
}
