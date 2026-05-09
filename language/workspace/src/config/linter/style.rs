use serde::Deserialize;

use super::{
    ArrayTypeStyle, ArrayTypeStyleJson, FilenameCase, FilenameCaseJson, GroupedAccessorPairsOrder,
    GroupedAccessorPairsOrderJson, LinterOptions, ObjectShorthandMode, ObjectShorthandModeJson,
    OperatorAssignmentMode, OperatorAssignmentModeJson, PreferConstDestructuring,
    PreferConstDestructuringJson, SortImportsMemberSyntax, SortImportsMemberSyntaxJson,
    TypeDefinitionStyle, TypeDefinitionStyleJson, YodaMode, YodaModeJson,
    validate_exact_enum_order, validate_regex_patterns,
};

/// Style-category linter options.
#[derive(Debug, Clone)]
pub struct LinterStyleOptions {
    /// Preferred array type syntax.
    pub array_type: ArrayTypeStyle,
    /// Preferred type definition syntax.
    pub type_definition_style: TypeDefinitionStyle,
    /// Required catch clause error name.
    pub catch_error_name: String,
    /// Required filename case style.
    pub filename_case: FilenameCase,
    /// Allow empty interfaces that extend exactly one supertype.
    pub allow_single_extends_empty_interface: bool,
    /// Allowed uppercase keyword prefixes for inline comments.
    pub comment_keywords: Vec<String>,
    /// Allowed tags for keyword comments.
    pub comment_keyword_tags: Vec<String>,
    /// Minimum non empty lines required for separator heading comments.
    pub comment_separator_heading_min_lines: usize,
    /// Allow named callbacks in `prefer-arrow-callback`.
    pub prefer_arrow_callback_allow_named_functions: bool,
    /// Allow unbound `this` in `prefer-arrow-callback`.
    pub prefer_arrow_callback_allow_unbound_this: bool,
    /// Allow `else if` chains in `no-else-return`.
    pub no_else_return_allow_else_if: bool,
    /// Allow keyword property names in `dot-notation`.
    pub dot_notation_allow_keywords: bool,
    /// Regex pattern of property names exempt from `dot-notation`.
    pub dot_notation_allow_pattern: Option<String>,
    /// Check nested boolean contexts in `no-extra-boolean-cast`.
    pub no_extra_boolean_cast_enforce_for_inner_expressions: bool,
    /// Ordering policy for `grouped-accessor-pairs`.
    pub grouped_accessor_pairs_order: GroupedAccessorPairsOrder,
    /// Enforce `grouped-accessor-pairs` in type-only member bodies.
    pub grouped_accessor_pairs_enforce_for_types: bool,
    /// Enforcement mode for `operator-assignment`.
    pub operator_assignment_mode: OperatorAssignmentMode,
    /// Enforcement mode for `object-shorthand`.
    pub object_shorthand_mode: ObjectShorthandMode,
    /// Allow quoted keys to stay longform in `object-shorthand`.
    pub object_shorthand_avoid_quotes: bool,
    /// Ignore constructor-like names in `object-shorthand`.
    pub object_shorthand_ignore_constructors: bool,
    /// Regex exemption for method names in `object-shorthand`.
    pub object_shorthand_methods_ignore_pattern: Option<String>,
    /// Avoid shorthand for explicit return arrow values in `object-shorthand`.
    pub object_shorthand_avoid_explicit_return_arrows: bool,
    /// Keep default-assignment ternaries in `no-unneeded-ternary`.
    pub no_unneeded_ternary_default_assignment: bool,
    /// Allow empty Promise.reject calls in `prefer-promise-reject-errors`.
    pub prefer_promise_reject_errors_allow_empty_reject: bool,
    /// Destructuring reporting policy for `prefer-const`.
    pub prefer_const_destructuring: PreferConstDestructuring,
    /// Ignore read-before-assign bindings in `prefer-const`.
    pub prefer_const_ignore_read_before_assign: bool,
    /// Enforcement mode for `yoda`.
    pub yoda_mode: YodaMode,
    /// Allow Yoda range tests in `yoda`.
    pub yoda_except_range: bool,
    /// Restrict `yoda` to equality operators.
    pub yoda_only_equality: bool,
    /// Ignore case in `sort-imports`.
    pub sort_imports_ignore_case: bool,
    /// Ignore declaration ordering in `sort-imports`.
    pub sort_imports_ignore_declaration_sort: bool,
    /// Ignore member ordering in `sort-imports`.
    pub sort_imports_ignore_member_sort: bool,
    /// Allow separated declaration groups in `sort-imports`.
    pub sort_imports_allow_separated_groups: bool,
    /// Member syntax ordering in `sort-imports`.
    pub sort_imports_member_syntax_sort_order: Vec<SortImportsMemberSyntax>,
    /// Prefer top-level `import type` in `consistent-type-imports`.
    pub consistent_type_imports_prefer_type_imports: bool,
    /// Prefer inline `type` specifiers in `consistent-type-imports`.
    pub consistent_type_imports_prefer_inline_type_imports: bool,
    /// Ignore conditional test positions in `prefer-nullish-coalescing`.
    pub prefer_nullish_coalescing_ignore_conditional_tests: bool,
    /// Ignore mixed logical expressions in `prefer-nullish-coalescing`.
    pub prefer_nullish_coalescing_ignore_mixed_logical_expressions: bool,
    /// Ignore ternary checks in `prefer-nullish-coalescing`.
    pub prefer_nullish_coalescing_ignore_ternary_tests: bool,
}

impl Default for LinterStyleOptions {
    fn default() -> Self {
        Self {
            array_type: ArrayTypeStyle::default(),
            type_definition_style: TypeDefinitionStyle::default(),
            catch_error_name: "error".to_string(),
            filename_case: FilenameCase::default(),
            allow_single_extends_empty_interface: false,
            comment_keywords: vec!["NOTE".to_string(), "TODO".to_string(), "FUGU".to_string()],
            comment_keyword_tags: vec![
                "#Performance".to_string(),
                "#Robustness".to_string(),
                "#Broken".to_string(),
                "#Cleanup".to_string(),
                "#Incomplete".to_string(),
                "#Suspicious".to_string(),
                "#Security".to_string(),
                "#Architecture".to_string(),
            ],
            comment_separator_heading_min_lines: 3,
            prefer_arrow_callback_allow_named_functions: false,
            prefer_arrow_callback_allow_unbound_this: true,
            no_else_return_allow_else_if: true,
            dot_notation_allow_keywords: true,
            dot_notation_allow_pattern: None,
            no_extra_boolean_cast_enforce_for_inner_expressions: false,
            grouped_accessor_pairs_order: GroupedAccessorPairsOrder::default(),
            grouped_accessor_pairs_enforce_for_types: false,
            operator_assignment_mode: OperatorAssignmentMode::default(),
            object_shorthand_mode: ObjectShorthandMode::default(),
            object_shorthand_avoid_quotes: false,
            object_shorthand_ignore_constructors: false,
            object_shorthand_methods_ignore_pattern: None,
            object_shorthand_avoid_explicit_return_arrows: false,
            no_unneeded_ternary_default_assignment: true,
            prefer_promise_reject_errors_allow_empty_reject: false,
            prefer_const_destructuring: PreferConstDestructuring::default(),
            prefer_const_ignore_read_before_assign: false,
            yoda_mode: YodaMode::default(),
            yoda_except_range: false,
            yoda_only_equality: false,
            sort_imports_ignore_case: false,
            sort_imports_ignore_declaration_sort: false,
            sort_imports_ignore_member_sort: false,
            sort_imports_allow_separated_groups: false,
            sort_imports_member_syntax_sort_order: vec![
                SortImportsMemberSyntax::None,
                SortImportsMemberSyntax::All,
                SortImportsMemberSyntax::Multiple,
                SortImportsMemberSyntax::Single,
            ],
            consistent_type_imports_prefer_type_imports: true,
            consistent_type_imports_prefer_inline_type_imports: false,
            prefer_nullish_coalescing_ignore_conditional_tests: true,
            prefer_nullish_coalescing_ignore_mixed_logical_expressions: false,
            prefer_nullish_coalescing_ignore_ternary_tests: false,
        }
    }
}

/// Style-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterStyleJson {
    /// Preferred array type syntax.
    pub array_type: Option<ArrayTypeStyleJson>,
    /// Preferred type definition syntax.
    pub type_definition_style: Option<TypeDefinitionStyleJson>,
    /// Required catch clause error name.
    pub catch_error_name: Option<String>,
    /// Required filename case style.
    pub filename_case: Option<FilenameCaseJson>,
    /// Allow empty interfaces that extend exactly one supertype.
    pub allow_single_extends_empty_interface: Option<bool>,
    /// Allow named callbacks in `prefer-arrow-callback`.
    pub prefer_arrow_callback_allow_named_functions: Option<bool>,
    /// Allow unbound `this` in `prefer-arrow-callback`.
    pub prefer_arrow_callback_allow_unbound_this: Option<bool>,
    /// Allow `else if` chains in `no-else-return`.
    pub no_else_return_allow_else_if: Option<bool>,
    /// Allow keyword property names in `dot-notation`.
    pub dot_notation_allow_keywords: Option<bool>,
    /// Regex pattern of property names exempt from `dot-notation`.
    pub dot_notation_allow_pattern: Option<String>,
    /// Check nested boolean contexts in `no-extra-boolean-cast`.
    pub no_extra_boolean_cast_enforce_for_inner_expressions: Option<bool>,
    /// Ordering policy for `grouped-accessor-pairs`.
    pub grouped_accessor_pairs_order: Option<GroupedAccessorPairsOrderJson>,
    /// Enforce `grouped-accessor-pairs` in type-only member bodies.
    pub grouped_accessor_pairs_enforce_for_types: Option<bool>,
    /// Enforcement mode for `operator-assignment`.
    pub operator_assignment_mode: Option<OperatorAssignmentModeJson>,
    /// Enforcement mode for `object-shorthand`.
    pub object_shorthand_mode: Option<ObjectShorthandModeJson>,
    /// Allow quoted keys to stay longform in `object-shorthand`.
    pub object_shorthand_avoid_quotes: Option<bool>,
    /// Ignore constructor-like names in `object-shorthand`.
    pub object_shorthand_ignore_constructors: Option<bool>,
    /// Regex exemption for method names in `object-shorthand`.
    pub object_shorthand_methods_ignore_pattern: Option<String>,
    /// Avoid shorthand for explicit return arrow values in `object-shorthand`.
    pub object_shorthand_avoid_explicit_return_arrows: Option<bool>,
    /// Keep default-assignment ternaries in `no-unneeded-ternary`.
    pub no_unneeded_ternary_default_assignment: Option<bool>,
    /// Allow empty Promise.reject calls in `prefer-promise-reject-errors`.
    pub prefer_promise_reject_errors_allow_empty_reject: Option<bool>,
    /// Destructuring reporting policy for `prefer-const`.
    pub prefer_const_destructuring: Option<PreferConstDestructuringJson>,
    /// Ignore read-before-assign bindings in `prefer-const`.
    pub prefer_const_ignore_read_before_assign: Option<bool>,
    /// Enforcement mode for `yoda`.
    pub yoda_mode: Option<YodaModeJson>,
    /// Allow Yoda range tests in `yoda`.
    pub yoda_except_range: Option<bool>,
    /// Restrict `yoda` to equality operators.
    pub yoda_only_equality: Option<bool>,
    /// Ignore case in `sort-imports`.
    pub sort_imports_ignore_case: Option<bool>,
    /// Ignore declaration ordering in `sort-imports`.
    pub sort_imports_ignore_declaration_sort: Option<bool>,
    /// Ignore member ordering in `sort-imports`.
    pub sort_imports_ignore_member_sort: Option<bool>,
    /// Allow separated declaration groups in `sort-imports`.
    pub sort_imports_allow_separated_groups: Option<bool>,
    /// Member syntax ordering in `sort-imports`.
    pub sort_imports_member_syntax_sort_order: Option<Vec<SortImportsMemberSyntaxJson>>,
    /// Prefer top-level `import type` in `consistent-type-imports`.
    pub consistent_type_imports_prefer_type_imports: Option<bool>,
    /// Prefer inline `type` specifiers in `consistent-type-imports`.
    pub consistent_type_imports_prefer_inline_type_imports: Option<bool>,
    /// Ignore conditional tests in `prefer-nullish-coalescing`.
    pub prefer_nullish_coalescing_ignore_conditional_tests: Option<bool>,
    /// Ignore mixed logical expressions in `prefer-nullish-coalescing`.
    pub prefer_nullish_coalescing_ignore_mixed_logical_expressions: Option<bool>,
    /// Ignore ternary checks in `prefer-nullish-coalescing`.
    pub prefer_nullish_coalescing_ignore_ternary_tests: Option<bool>,
}

impl LinterStyleJson {
    /// Validate style-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        validate_regex_patterns(
            "linter.dotNotationAllowPattern",
            self.dot_notation_allow_pattern
                .as_ref()
                .map(std::slice::from_ref),
        )?;
        validate_regex_patterns(
            "linter.objectShorthandMethodsIgnorePattern",
            self.object_shorthand_methods_ignore_pattern
                .as_ref()
                .map(std::slice::from_ref),
        )?;
        validate_exact_enum_order(
            "linter.sortImportsMemberSyntaxSortOrder",
            self.sort_imports_member_syntax_sort_order.as_deref(),
            &[
                SortImportsMemberSyntaxJson::None,
                SortImportsMemberSyntaxJson::All,
                SortImportsMemberSyntaxJson::Multiple,
                SortImportsMemberSyntaxJson::Single,
            ],
        )?;

        Ok(())
    }

    /// Apply style-category options to one linter options struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(array_type) = self.array_type {
            options.style.array_type = array_type.into();
        }

        if let Some(type_definition_style) = self.type_definition_style {
            options.style.type_definition_style = type_definition_style.into();
        }

        if let Some(ref catch_error_name) = self.catch_error_name {
            options.style.catch_error_name = catch_error_name.clone();
        }

        if let Some(filename_case) = self.filename_case {
            options.style.filename_case = filename_case.into();
        }

        if let Some(allow_single_extends_empty_interface) =
            self.allow_single_extends_empty_interface
        {
            options.style.allow_single_extends_empty_interface =
                allow_single_extends_empty_interface;
        }

        if let Some(prefer_arrow_callback_allow_named_functions) =
            self.prefer_arrow_callback_allow_named_functions
        {
            options.style.prefer_arrow_callback_allow_named_functions =
                prefer_arrow_callback_allow_named_functions;
        }

        if let Some(prefer_arrow_callback_allow_unbound_this) =
            self.prefer_arrow_callback_allow_unbound_this
        {
            options.style.prefer_arrow_callback_allow_unbound_this =
                prefer_arrow_callback_allow_unbound_this;
        }

        if let Some(no_else_return_allow_else_if) = self.no_else_return_allow_else_if {
            options.style.no_else_return_allow_else_if = no_else_return_allow_else_if;
        }

        if let Some(dot_notation_allow_keywords) = self.dot_notation_allow_keywords {
            options.style.dot_notation_allow_keywords = dot_notation_allow_keywords;
        }

        if let Some(ref dot_notation_allow_pattern) = self.dot_notation_allow_pattern {
            options.style.dot_notation_allow_pattern = Some(dot_notation_allow_pattern.clone());
        }

        if let Some(no_extra_boolean_cast_enforce_for_inner_expressions) =
            self.no_extra_boolean_cast_enforce_for_inner_expressions
        {
            options
                .style
                .no_extra_boolean_cast_enforce_for_inner_expressions =
                no_extra_boolean_cast_enforce_for_inner_expressions;
        }

        if let Some(grouped_accessor_pairs_order) = self.grouped_accessor_pairs_order {
            options.style.grouped_accessor_pairs_order = grouped_accessor_pairs_order.into();
        }

        if let Some(grouped_accessor_pairs_enforce_for_types) =
            self.grouped_accessor_pairs_enforce_for_types
        {
            options.style.grouped_accessor_pairs_enforce_for_types =
                grouped_accessor_pairs_enforce_for_types;
        }

        if let Some(operator_assignment_mode) = self.operator_assignment_mode {
            options.style.operator_assignment_mode = operator_assignment_mode.into();
        }

        if let Some(object_shorthand_mode) = self.object_shorthand_mode {
            options.style.object_shorthand_mode = object_shorthand_mode.into();
        }

        if let Some(object_shorthand_avoid_quotes) = self.object_shorthand_avoid_quotes {
            options.style.object_shorthand_avoid_quotes = object_shorthand_avoid_quotes;
        }

        if let Some(object_shorthand_ignore_constructors) =
            self.object_shorthand_ignore_constructors
        {
            options.style.object_shorthand_ignore_constructors =
                object_shorthand_ignore_constructors;
        }

        if let Some(ref object_shorthand_methods_ignore_pattern) =
            self.object_shorthand_methods_ignore_pattern
        {
            options.style.object_shorthand_methods_ignore_pattern =
                Some(object_shorthand_methods_ignore_pattern.clone());
        }

        if let Some(object_shorthand_avoid_explicit_return_arrows) =
            self.object_shorthand_avoid_explicit_return_arrows
        {
            options.style.object_shorthand_avoid_explicit_return_arrows =
                object_shorthand_avoid_explicit_return_arrows;
        }

        if let Some(no_unneeded_ternary_default_assignment) =
            self.no_unneeded_ternary_default_assignment
        {
            options.style.no_unneeded_ternary_default_assignment =
                no_unneeded_ternary_default_assignment;
        }

        if let Some(prefer_promise_reject_errors_allow_empty_reject) =
            self.prefer_promise_reject_errors_allow_empty_reject
        {
            options
                .style
                .prefer_promise_reject_errors_allow_empty_reject =
                prefer_promise_reject_errors_allow_empty_reject;
        }

        if let Some(prefer_const_destructuring) = self.prefer_const_destructuring {
            options.style.prefer_const_destructuring = prefer_const_destructuring.into();
        }

        if let Some(prefer_const_ignore_read_before_assign) =
            self.prefer_const_ignore_read_before_assign
        {
            options.style.prefer_const_ignore_read_before_assign =
                prefer_const_ignore_read_before_assign;
        }

        if let Some(yoda_mode) = self.yoda_mode {
            options.style.yoda_mode = yoda_mode.into();
        }

        if let Some(yoda_except_range) = self.yoda_except_range {
            options.style.yoda_except_range = yoda_except_range;
        }

        if let Some(yoda_only_equality) = self.yoda_only_equality {
            options.style.yoda_only_equality = yoda_only_equality;
        }

        if let Some(sort_imports_ignore_case) = self.sort_imports_ignore_case {
            options.style.sort_imports_ignore_case = sort_imports_ignore_case;
        }

        if let Some(sort_imports_ignore_declaration_sort) =
            self.sort_imports_ignore_declaration_sort
        {
            options.style.sort_imports_ignore_declaration_sort =
                sort_imports_ignore_declaration_sort;
        }

        if let Some(sort_imports_ignore_member_sort) = self.sort_imports_ignore_member_sort {
            options.style.sort_imports_ignore_member_sort = sort_imports_ignore_member_sort;
        }

        if let Some(sort_imports_allow_separated_groups) = self.sort_imports_allow_separated_groups
        {
            options.style.sort_imports_allow_separated_groups = sort_imports_allow_separated_groups;
        }

        if let Some(ref sort_imports_member_syntax_sort_order) =
            self.sort_imports_member_syntax_sort_order
        {
            options.style.sort_imports_member_syntax_sort_order =
                sort_imports_member_syntax_sort_order
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect();
        }

        if let Some(consistent_type_imports_prefer_type_imports) =
            self.consistent_type_imports_prefer_type_imports
        {
            options.style.consistent_type_imports_prefer_type_imports =
                consistent_type_imports_prefer_type_imports;
        }

        if let Some(consistent_type_imports_prefer_inline_type_imports) =
            self.consistent_type_imports_prefer_inline_type_imports
        {
            options
                .style
                .consistent_type_imports_prefer_inline_type_imports =
                consistent_type_imports_prefer_inline_type_imports;
        }

        if let Some(prefer_nullish_coalescing_ignore_conditional_tests) =
            self.prefer_nullish_coalescing_ignore_conditional_tests
        {
            options
                .style
                .prefer_nullish_coalescing_ignore_conditional_tests =
                prefer_nullish_coalescing_ignore_conditional_tests;
        }

        if let Some(prefer_nullish_coalescing_ignore_mixed_logical_expressions) =
            self.prefer_nullish_coalescing_ignore_mixed_logical_expressions
        {
            options
                .style
                .prefer_nullish_coalescing_ignore_mixed_logical_expressions =
                prefer_nullish_coalescing_ignore_mixed_logical_expressions;
        }

        if let Some(prefer_nullish_coalescing_ignore_ternary_tests) =
            self.prefer_nullish_coalescing_ignore_ternary_tests
        {
            options.style.prefer_nullish_coalescing_ignore_ternary_tests =
                prefer_nullish_coalescing_ignore_ternary_tests;
        }
    }
}
