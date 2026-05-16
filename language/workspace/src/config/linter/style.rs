use serde::{Deserialize, Serialize};

use super::{
    ArrayTypeStyle, FilenameCase, GroupedAccessorPairsOrder, ObjectShorthandMode,
    OperatorAssignmentMode, PreferConstDestructuring, SortImportsMemberSyntax, TypeDefinitionStyle,
    YodaMode,
};

/// Style-category linter options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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
