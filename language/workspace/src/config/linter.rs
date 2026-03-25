use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;

use crate::{DiagnosticPolicy, DiagnosticPolicyJson};

/// Lint rule categories.
///
/// Each category has a letter code used in lint identifiers (e.g., `LC002` for Correctness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintCategory {
    /// Correctness (C) lints detect likely bugs and logic errors.
    /// These are high-confidence issues that are almost always wrong.
    Correctness,

    /// Suspicious (U) lints detect code that is likely unintentional.
    /// These patterns are usually bugs but may occasionally be intentional.
    Suspicious,

    /// Performance (P) lints detect inefficient patterns.
    /// The code is correct but could be faster or use less memory.
    Performance,

    /// Style (Y) lints enforce consistent coding style.
    /// These are subjective preferences, not correctness issues.
    Style,

    /// Security (S) lints detect potential vulnerabilities.
    /// These patterns may expose the application to attacks.
    Security,

    /// Complexity (X) lints detect overly complex code.
    /// High complexity makes code harder to understand and maintain.
    Complexity,

    /// Restriction (R) lints enforce project-specific restrictions.
    /// These are opt-in rules that ban certain patterns by choice.
    Restriction,
}

impl LintCategory {
    /// Get the category letter for diagnostic codes.
    pub const fn letter(&self) -> char {
        match self {
            Self::Correctness => 'C',
            Self::Suspicious => 'U',
            Self::Performance => 'P',
            Self::Style => 'Y',
            Self::Security => 'S',
            Self::Complexity => 'X',
            Self::Restriction => 'R',
        }
    }

    /// Get the category name.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Suspicious => "suspicious",
            Self::Performance => "performance",
            Self::Style => "style",
            Self::Security => "security",
            Self::Complexity => "complexity",
            Self::Restriction => "restriction",
        }
    }

    /// Get the description of the category.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Correctness => "detects likely bugs and logic errors",
            Self::Suspicious => "detects potentially unintentional code",
            Self::Performance => "detects inefficient patterns",
            Self::Style => "enforces consistent coding style",
            Self::Security => "detects potential security vulnerabilities",
            Self::Complexity => "detects overly complex code",
            Self::Restriction => "enforces specific  restrictions",
        }
    }

    /// Whether this category is part of the "recommended" set.
    pub const fn is_recommended(&self) -> bool {
        matches!(self, Self::Correctness | Self::Suspicious | Self::Security)
    }

    /// Default severity for rules in this category.
    pub const fn default_severity(&self) -> LintSeverity {
        match self {
            Self::Correctness => LintSeverity::Error,
            Self::Suspicious => LintSeverity::Warning,
            Self::Performance => LintSeverity::Warning,
            Self::Style => LintSeverity::Warning,
            Self::Security => LintSeverity::Error,
            Self::Complexity => LintSeverity::Warning,
            Self::Restriction => LintSeverity::Note, // opt-in
        }
    }

    /// All categories.
    pub const ALL: &'static [LintCategory] = &[
        Self::Correctness,
        Self::Suspicious,
        Self::Performance,
        Self::Style,
        Self::Security,
        Self::Complexity,
        Self::Restriction,
    ];
}

impl std::fmt::Display for LintCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Lint rule preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LintPreset {
    /// No rules enabled by default.
    None,
    /// Recommended rules enabled.
    #[default]
    Recommended,
    /// Recommended plus strict rules enabled.
    Strict,
    /// All rules enabled.
    All,
}

impl LintPreset {
    /// Parse a preset from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" | "off" => Some(Self::None),
            "recommended" => Some(Self::Recommended),
            "strict" => Some(Self::Strict),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Recommended => "recommended",
            Self::Strict => "strict",
            Self::All => "all",
        }
    }
}

impl std::fmt::Display for LintPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Preferred array type syntax for the `array-type` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ArrayTypeStyle {
    /// Prefer `T[]` syntax.
    #[default]
    Array,
    /// Prefer `Array<T>` syntax.
    Generic,
}

/// Preferred type definition syntax for the `consistent-type-definitions` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TypeDefinitionStyle {
    /// Prefer `type` aliases.
    #[default]
    Type,
    /// Prefer `interface` declarations.
    Interface,
}

/// Filename case style for the `filename-case` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FilenameCase {
    /// kebab-case (e.g., `my-component.ts`).
    #[default]
    Kebab,
    /// snake_case (e.g., `my_component.ts`).
    Snake,
    /// camelCase (e.g., `myComponent.ts`).
    Camel,
    /// PascalCase (e.g., `MyComponent.ts`).
    Pascal,
}

/// Return-await mode for the `return-await` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReturnAwaitMode {
    /// Require await only in error handling contexts and forbid it elsewhere.
    #[default]
    InTryCatch,
    /// Require await only in error handling contexts and do not enforce elsewhere.
    ErrorHandlingCorrectnessOnly,
    /// Require await in all contexts.
    Always,
    /// Forbid await in all contexts.
    Never,
}

/// Required Unicode regex flag for `require-unicode-regexp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UnicodeRegexpRequireFlag {
    /// Require the `u` flag.
    #[default]
    U,
    /// Require the `v` flag.
    V,
}

impl UnicodeRegexpRequireFlag {
    /// Return the required flag character.
    pub const fn as_char(self) -> char {
        match self {
            Self::U => 'u',
            Self::V => 'v',
        }
    }
}

/// Switch counting variant for `cyclomatic-complexity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CyclomaticComplexityVariant {
    /// Count each non-default switch case as a branch.
    #[default]
    Classic,
    /// Count each switch as a single branch regardless of case count.
    Modified,
}

/// `this` parameter counting policy for `max-params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MaxParamsCountThis {
    /// Never count the `this` parameter.
    Never,
    /// Count `this` unless it is explicitly typed as `void`.
    #[default]
    ExceptVoid,
    /// Always count the `this` parameter.
    Always,
}

/// Enforcement mode for `eqeqeq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EqeqeqMode {
    /// Always require strict equality operators.
    #[default]
    Always,
    /// Allow loose equality for null checks, typeof checks, and same type literals.
    Smart,
    /// Allow loose equality only for null checks.
    AllowNull,
}

/// Null comparison policy for `eqeqeq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EqeqeqNullPolicy {
    /// Require strict null equality.
    #[default]
    Always,
    /// Require loose null equality.
    Never,
    /// Ignore null equality style.
    Ignore,
}

/// Enforcement mode for `operator-assignment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OperatorAssignmentMode {
    /// Require shorthand assignment where possible.
    #[default]
    Always,
    /// Disallow shorthand assignment operators.
    Never,
}

/// Enforcement mode for `object-shorthand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ObjectShorthandMode {
    /// Require shorthand for methods and properties.
    #[default]
    Always,
    /// Require shorthand for methods only.
    Methods,
    /// Require shorthand for properties only.
    Properties,
    /// Disallow shorthand methods and properties.
    Never,
    /// Require object literals to use consistent shorthand style.
    Consistent,
    /// Require shorthand only when every eligible property can use it.
    ConsistentAsNeeded,
}

/// Enforcement mode for `yoda`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum YodaMode {
    /// Require literal comparisons in Yoda form.
    Always,
    /// Disallow literal comparisons in Yoda form.
    #[default]
    Never,
}

/// Ordering policy for `grouped-accessor-pairs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GroupedAccessorPairsOrder {
    /// Allow either adjacent accessor order.
    #[default]
    AnyOrder,
    /// Require getters before setters.
    GetBeforeSet,
    /// Require setters before getters.
    SetBeforeGet,
}

/// Member syntax groups for `sort-imports`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortImportsMemberSyntax {
    /// Side-effect import syntax.
    None,
    /// Namespace import syntax.
    All,
    /// Multiple named imports.
    Multiple,
    /// Single default or named import.
    Single,
}

/// Destructuring policy for `prefer-const`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PreferConstDestructuring {
    /// Report any const eligible binding in a destructuring.
    #[default]
    Any,
    /// Report destructuring bindings only when all are const eligible.
    All,
}

/// Warning comment term matching location for `no-warning-comments`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WarningCommentLocation {
    /// Match terms only at the logical start of the comment.
    #[default]
    Start,
    /// Match terms anywhere in the comment body.
    Anywhere,
}

/// Bitwise operators configurable for the `no-bitwise` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitwiseOperator {
    /// `&`
    And,
    /// `^`
    Xor,
    /// `|`
    Or,
    /// `~`
    Not,
    /// `<<`
    ShiftLeft,
    /// `<<|`
    SaturatingShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    UnsignedShiftRight,
    /// `&=`
    AndAssign,
    /// `^=`
    XorAssign,
    /// `|=`
    OrAssign,
    /// `<<=`
    ShiftLeftAssign,
    /// `<<|=`
    SaturatingShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
    /// `>>>=`
    UnsignedShiftRightAssign,
}

impl BitwiseOperator {
    /// Return the operator text used in diagnostics and configuration.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::And => "&",
            Self::Xor => "^",
            Self::Or => "|",
            Self::Not => "~",
            Self::ShiftLeft => "<<",
            Self::SaturatingShiftLeft => "<<|",
            Self::ShiftRight => ">>",
            Self::UnsignedShiftRight => ">>>",
            Self::AndAssign => "&=",
            Self::XorAssign => "^=",
            Self::OrAssign => "|=",
            Self::ShiftLeftAssign => "<<=",
            Self::SaturatingShiftLeftAssign => "<<|=",
            Self::ShiftRightAssign => ">>=",
            Self::UnsignedShiftRightAssign => ">>>=",
        }
    }
}

/// Module boundary lint options for module boundary aware rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintModuleBoundariesOptions {
    /// Policy for modules that do not match any configured component.
    pub unknown_component_policy: DiagnosticPolicy,
    /// Declared components and their path match patterns.
    pub components: Vec<LintModuleComponent>,
    /// Allowed component to component dependency rules.
    pub dependency_rules: Vec<LintModuleDependencyRule>,
    /// Explicit dependency exceptions.
    pub exceptions: Vec<LintModuleDependencyException>,
}

impl Default for LintModuleBoundariesOptions {
    fn default() -> Self {
        Self {
            unknown_component_policy: DiagnosticPolicy::Allow,
            components: Vec::new(),
            dependency_rules: Vec::new(),
            exceptions: Vec::new(),
        }
    }
}

/// One module component declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintModuleComponent {
    /// The unique component name.
    pub name: String,
    /// Glob patterns used to match module paths into this component.
    pub path_patterns: Vec<String>,
}

/// One allowed dependency rule between module components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintModuleDependencyRule {
    /// The source component name.
    pub from: String,
    /// Destination components this source component may import.
    pub allow: Vec<String>,
}

/// One module dependency exception for specific module path patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintModuleDependencyException {
    /// The source component name for this exception.
    pub from: String,
    /// The destination component name for this exception.
    pub to: String,
    /// Module path patterns where this exception is allowed.
    pub path_patterns: Vec<String>,
    /// Optional human-readable reason for this exception.
    pub reason: Option<String>,
}

/// Linter options.
///
/// Rule severity resolution order (highest precedence first):
/// 1. Individual rule overrides
/// 2. Category-level overrides
/// 3. Preset defaults
#[derive(Debug, Clone)]
pub struct LinterOptions {
    /// Whether linting is enabled.
    pub enabled: bool,
    /// Base preset (none, recommended, all).
    pub preset: LintPreset,
    /// Category-level severity overrides.
    pub categories: IndexMap<LintCategory, LintSeverity>,
    /// Individual rule severity overrides.
    pub overrides: IndexMap<String, LintSeverity>,
    /// Include declaration files when evaluating declaration-gated rules.
    pub include_declaration_files: bool,
    /// Allow explicit `void` to intentionally discard Promise results.
    pub allow_void_discard: bool,
    /// Allow explicit `void` returns in `no-promise-executor-return`.
    pub no_promise_executor_return_allow_void: bool,
    /// Check callback positions in `no-misused-promises`.
    pub check_misused_promises_in_callbacks: bool,
    /// Check conditionals in `no-misused-promises`.
    pub check_misused_promises_in_conditionals: bool,
    /// Check spread positions in `no-misused-promises`.
    pub check_misused_promises_in_spreads: bool,
    /// Check switch discriminants and cases in `use-isnan`.
    pub use_isnan_enforce_switch_case: bool,
    /// Check `indexOf` and `lastIndexOf` calls in `use-isnan`.
    pub use_isnan_enforce_index_of: bool,
    /// Check property assignments in `no-self-assign`.
    pub no_self_assign_check_properties: bool,
    /// Ignore explicit `void` wrappers in `no-confusing-void-expression`.
    pub no_confusing_void_expression_ignore_void_operator: bool,
    /// Ignore returned void expressions inside void-returning functions.
    pub no_confusing_void_expression_ignore_void_returning_functions: bool,
    /// Check nested `var` declarations in `no-inner-declarations`.
    pub no_inner_declarations_check_var_declarations: bool,
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
    /// Required Unicode regex flag for `require-unicode-regexp`.
    pub require_unicode_regexp_require_flag: UnicodeRegexpRequireFlag,
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
    /// Allow `rel="noreferrer"` without `noopener` in `no-blank-target`.
    pub no_blank_target_allow_no_referrer: bool,
    /// Domains allowed to use `target="_blank"` without rel hardening.
    pub no_blank_target_allow_domains: Vec<String>,
    /// Entropy threshold in tenths for `no-secrets`.
    pub no_secrets_entropy_threshold: u32,

    // complexity thresholds
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
    /// Maximum static parameters (generics including const values).
    pub max_static_params: usize,
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
    /// Include immediately invoked functions in `max-lines-per-function`.
    pub max_lines_per_function_include_iifes: bool,
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
    /// Maximum type complexity (nesting depth of generics/unions/intersections).
    pub max_type_complexity: usize,
    /// Maximum occurrences of the same string literal before warning.
    pub max_duplicate_string_occurrences: usize,
    /// Minimum lines required to consider a block for duplicate code checks.
    pub min_duplicate_code_lines: usize,
    /// Minimum tokens required to consider a block for duplicate code checks.
    pub min_duplicate_code_tokens: usize,
    /// Minimum similarity percent for near duplicate code matching (0-100).
    /// A value of 0 disables near duplicate matching.
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

    // style options
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
    pub allow_named_functions_in_prefer_arrow_callback: bool,
    /// Allow unbound `this` in `prefer-arrow-callback`.
    pub allow_unbound_this_in_prefer_arrow_callback: bool,
    /// Allow `else if` chains in `no-else-return`.
    pub no_else_return_allow_else_if: bool,
    /// Allow keyword property names in `dot-notation`.
    pub dot_notation_allow_keywords: bool,
    /// Regex pattern of property names exempt from `dot-notation`.
    pub dot_notation_allow_pattern: Option<String>,
    /// Check nested boolean contexts in `no-extra-boolean-cast`.
    pub no_extra_boolean_cast_enforce_for_inner_expressions: bool,
    /// Enforcement mode for `eqeqeq`.
    pub eqeqeq_mode: EqeqeqMode,
    /// Ordering policy for `grouped-accessor-pairs`.
    pub grouped_accessor_pairs_order: GroupedAccessorPairsOrder,
    /// Enforce `grouped-accessor-pairs` in type-only member bodies.
    pub grouped_accessor_pairs_enforce_for_types: bool,
    /// Null comparison policy for `eqeqeq`.
    pub eqeqeq_null: EqeqeqNullPolicy,
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
    /// Disallow `import("...")` type annotations in `consistent-type-imports`.
    pub consistent_type_imports_disallow_type_annotations: bool,
    /// Ignore conditional test positions in `prefer-nullish-coalescing`.
    pub ignore_conditional_tests_in_prefer_nullish_coalescing: bool,
    /// Ignore mixed logical expressions in `prefer-nullish-coalescing`.
    pub ignore_mixed_logical_expressions_in_prefer_nullish_coalescing: bool,
    /// Ignore ternary checks in `prefer-nullish-coalescing`.
    pub ignore_ternary_tests_in_prefer_nullish_coalescing: bool,

    // restriction options
    /// Bitwise operators allowed by `no-bitwise`.
    pub allowed_bitwise_operators: Vec<BitwiseOperator>,
    /// Allow `x | 0` int32 cast hints in `no-bitwise`.
    pub allow_bitwise_int32_hint: bool,
    /// Console methods allowed by `no-console`.
    pub allowed_console_methods: Vec<String>,
    /// Ignore explicit `any` in variadic parameter types for `no-explicit-any`.
    pub ignore_explicit_any_in_rest_args: bool,
    /// Allow labels on loop statements in `no-labels`.
    pub allow_loop_labels: bool,
    /// Allow labels on switch statements in `no-labels`.
    pub allow_switch_labels: bool,
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Vec<f64>,
    /// Check strict `=== null` and `!== null` comparisons in `no-null`.
    pub check_strict_null_equality: bool,
    /// Allow `declare namespace` and `declare module foo {}` in `no-namespace`.
    pub allow_namespace_declarations: bool,
    /// Allow namespace declarations in definition files for `no-namespace`.
    pub allow_namespace_definition_files: bool,
    /// Allow `++` and `--` in for-loop afterthoughts for `no-plusplus`.
    pub allow_plusplus_for_loop_afterthoughts: bool,
    /// Allow static require target patterns for `no-require-imports`.
    pub allowed_require_import_patterns: Vec<String>,
    /// Allow `import foo = require("foo")` for `no-require-imports`.
    pub allow_require_import_aliases: bool,
    /// Where `no-warning-comments` should match terms.
    pub warning_comment_location: WarningCommentLocation,
    /// Decoration characters to ignore at the start of `no-warning-comments`.
    pub warning_comment_decoration: Vec<String>,
    /// Globals to restrict.
    pub restricted_globals: Vec<String>,
    /// Import paths to restrict.
    pub restricted_imports: Vec<String>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Vec<String>,
    /// Module boundary constraints for module boundary aware lints.
    pub module_boundaries: LintModuleBoundariesOptions,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: LintPreset::Recommended,
            categories: IndexMap::new(),
            overrides: IndexMap::new(),
            include_declaration_files: false,
            allow_void_discard: true,
            no_promise_executor_return_allow_void: false,
            check_misused_promises_in_callbacks: true,
            check_misused_promises_in_conditionals: true,
            check_misused_promises_in_spreads: true,
            use_isnan_enforce_switch_case: true,
            use_isnan_enforce_index_of: false,
            no_self_assign_check_properties: true,
            no_confusing_void_expression_ignore_void_operator: false,
            no_confusing_void_expression_ignore_void_returning_functions: false,
            no_inner_declarations_check_var_declarations: true,
            no_cond_assign_mode: ConditionAssignmentMode::default(),
            no_empty_allow_empty_catch: false,
            no_empty_pattern_allow_object_patterns_as_parameters: false,
            no_empty_function_allow: Vec::new(),
            no_fallthrough_allow_empty_case: false,
            no_fallthrough_comment_pattern: None,
            no_fallthrough_report_unused_comment: false,
            return_await_mode: ReturnAwaitMode::default(),
            require_unicode_regexp_require_flag: UnicodeRegexpRequireFlag::default(),
            ignored_unused_parameter_prefixes: vec!["_".to_string()],
            no_useless_rename_ignore_destructuring: false,
            no_useless_rename_ignore_import: false,
            no_useless_rename_ignore_export: false,
            no_useless_escape_allow_regex_characters: Vec::new(),
            no_blank_target_allow_no_referrer: true,
            no_blank_target_allow_domains: Vec::new(),
            no_secrets_entropy_threshold: 41,
            // complexity
            max_booleans: 3,
            max_branching_factor: 10,
            max_cognitive_complexity: 30,
            max_cyclomatic_complexity: 40,
            cyclomatic_complexity_variant: CyclomaticComplexityVariant::default(),
            max_depth: 4,
            max_static_params: 4,
            max_lines: 500,
            max_lines_skip_comments: false,
            max_lines_skip_blank_lines: false,
            max_lines_per_function: 50,
            max_lines_per_function_skip_comments: false,
            max_lines_per_function_skip_blank_lines: false,
            max_lines_per_function_include_iifes: false,
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
            // style
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
            allow_named_functions_in_prefer_arrow_callback: false,
            allow_unbound_this_in_prefer_arrow_callback: true,
            no_else_return_allow_else_if: true,
            dot_notation_allow_keywords: true,
            dot_notation_allow_pattern: None,
            no_extra_boolean_cast_enforce_for_inner_expressions: false,
            eqeqeq_mode: EqeqeqMode::default(),
            grouped_accessor_pairs_order: GroupedAccessorPairsOrder::default(),
            grouped_accessor_pairs_enforce_for_types: false,
            eqeqeq_null: EqeqeqNullPolicy::default(),
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
            consistent_type_imports_disallow_type_annotations: true,
            ignore_conditional_tests_in_prefer_nullish_coalescing: true,
            ignore_mixed_logical_expressions_in_prefer_nullish_coalescing: false,
            ignore_ternary_tests_in_prefer_nullish_coalescing: false,
            // restriction
            allowed_bitwise_operators: Vec::new(),
            allow_bitwise_int32_hint: false,
            allowed_console_methods: Vec::new(),
            ignore_explicit_any_in_rest_args: false,
            allow_loop_labels: false,
            allow_switch_labels: false,
            allowed_magic_numbers: vec![-1.0, 0.0, 1.0, 2.0],
            check_strict_null_equality: true,
            allow_namespace_declarations: false,
            allow_namespace_definition_files: true,
            allow_plusplus_for_loop_afterthoughts: false,
            allowed_require_import_patterns: Vec::new(),
            allow_require_import_aliases: false,
            warning_comment_location: WarningCommentLocation::Start,
            warning_comment_decoration: Vec::new(),
            restricted_globals: Vec::new(),
            restricted_imports: Vec::new(),
            warning_comment_terms: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "HACK".to_string(),
            ],
            module_boundaries: LintModuleBoundariesOptions::default(),
        }
    }
}

impl LinterOptions {
    /// Create options with recommended preset.
    pub fn recommended() -> Self {
        Self::default()
    }

    /// Create options with no rules enabled.
    pub fn none() -> Self {
        Self {
            preset: LintPreset::None,
            ..Self::default()
        }
    }

    /// Create options with all rules enabled.
    pub fn all() -> Self {
        Self {
            preset: LintPreset::All,
            ..Self::default()
        }
    }

    /// Set the preset.
    pub fn with_preset(mut self, preset: LintPreset) -> Self {
        self.preset = preset;
        self
    }

    /// Set a category's severity.
    pub fn with_category(mut self, category: LintCategory, severity: LintSeverity) -> Self {
        self.categories.insert(category, severity);
        self
    }

    /// Set a rule's severity.
    pub fn with_rule(mut self, rule: impl Into<String>, severity: LintSeverity) -> Self {
        self.overrides.insert(rule.into(), severity);
        self
    }

    /// Include declaration files when evaluating declaration-gated rules.
    pub fn with_include_declaration_files(mut self, include: bool) -> Self {
        self.include_declaration_files = include;
        self
    }

    /// Get a rule's configured severity (returns None if not overridden).
    pub fn get_rule_severity(&self, rule: &str) -> Option<LintSeverity> {
        self.overrides.get(rule).copied()
    }

    /// Set prefixes ignored by `no-unused-parameters`.
    pub fn with_ignored_unused_parameter_prefixes(
        mut self,
        prefixes: impl IntoIterator<Item = String>,
    ) -> Self {
        self.ignored_unused_parameter_prefixes = prefixes.into_iter().collect();
        self
    }

    /// Set allowed keyword prefixes for comment keyword comments.
    pub fn with_comment_keywords(mut self, keywords: impl IntoIterator<Item = String>) -> Self {
        self.comment_keywords = keywords.into_iter().collect();
        self
    }

    /// Set allowed tags for comment keyword comments.
    pub fn with_comment_keyword_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.comment_keyword_tags = tags.into_iter().collect();
        self
    }

    /// Set the minimum line count for separator heading comments.
    pub fn with_comment_separator_heading_min_lines(mut self, min_lines: usize) -> Self {
        self.comment_separator_heading_min_lines = min_lines;
        self
    }

    /// Get a category's configured severity (returns None if not overridden).
    pub fn get_category_severity(&self, category: LintCategory) -> Option<LintSeverity> {
        self.categories.get(&category).copied()
    }

    /// Resolve effective severity for one rule.
    ///
    /// Resolution order: rule override > category override > preset default.
    pub fn resolve_severity(
        &self,
        rule_id: &str,
        category: LintCategory,
        default: LintSeverity,
        is_recommended: bool,
        is_strict: bool,
    ) -> LintSeverity {
        // rule override takes precedence
        if let Some(severity) = self.overrides.get(rule_id) {
            return *severity;
        }

        // category override
        if let Some(severity) = self.categories.get(&category) {
            return *severity;
        }

        // preset logic
        match self.preset {
            LintPreset::None => LintSeverity::Off,
            LintPreset::Recommended => {
                if is_recommended {
                    default
                } else {
                    LintSeverity::Off
                }
            }
            LintPreset::Strict => {
                if is_strict {
                    default
                } else {
                    LintSeverity::Off
                }
            }
            LintPreset::All => default,
        }
    }
}

/// Rule severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LintSeverity {
    /// Rule is disabled.
    Off,
    /// Rule produces notes.
    Note,
    /// Rule produces warnings.
    #[default]
    Warning,
    /// Rule produces errors.
    Error,
}

impl LintSeverity {
    /// Whether this severity is enabled (not off).
    pub fn is_enabled(&self) -> bool {
        !matches!(self, LintSeverity::Off)
    }

    /// Whether this severity is a note.
    pub fn is_note(&self) -> bool {
        matches!(self, LintSeverity::Note)
    }

    /// Whether this severity is a warning.
    pub fn is_warn(&self) -> bool {
        matches!(self, LintSeverity::Warning)
    }

    /// Whether this severity is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, LintSeverity::Error)
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            LintSeverity::Off => "off",
            LintSeverity::Note => "note",
            LintSeverity::Warning => "warn",
            LintSeverity::Error => "error",
        }
    }

    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "off" | "none" | "0" => Some(Self::Off),
            "note" | "info" => Some(Self::Note),
            "warn" | "warning" | "1" => Some(Self::Warning),
            "error" | "deny" | "2" => Some(Self::Error),
            _ => None,
        }
    }
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Linter options (top-level, like Biome/Deno).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterJson {
    /// Whether linting is enabled. Default: true.
    pub enabled: Option<bool>,
    /// Rule configuration.
    #[serde(default)]
    pub rules: LinterRulesJson,

    // complexity thresholds
    /// Maximum boolean parameters or fields.
    pub max_booleans: Option<usize>,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: Option<usize>,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: Option<usize>,
    /// Switch counting variant for `cyclomatic-complexity`.
    pub cyclomatic_complexity_variant: Option<CyclomaticComplexityVariantJson>,
    /// Maximum nesting depth.
    pub max_depth: Option<usize>,
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
    /// Include immediately invoked functions in `max-lines-per-function`.
    pub max_lines_per_function_include_iifes: Option<bool>,
    /// Maximum callback nesting.
    pub max_nested_callbacks: Option<usize>,
    /// Maximum function parameters.
    pub max_params: Option<usize>,
    /// Legacy alias for void-this counting in `max-params`.
    pub max_params_count_void_this: Option<bool>,
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
    /// Maximum type complexity (nesting depth of generics/unions/intersections).
    pub max_type_complexity: Option<usize>,
    /// Maximum occurrences of the same string literal before warning.
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

    // style options
    /// Preferred array type syntax: "array" or "generic".
    pub array_type: Option<ArrayTypeStyleJson>,
    /// Preferred type definition syntax: "type" or "interface".
    pub type_definition_style: Option<TypeDefinitionStyleJson>,
    /// Required catch clause error name.
    pub catch_error_name: Option<String>,
    /// Required filename case style.
    pub filename_case: Option<FilenameCaseJson>,
    /// Allow empty interfaces that extend exactly one supertype.
    pub allow_single_extends_empty_interface: Option<bool>,
    /// Await policy for the `return-await` rule.
    pub return_await_mode: Option<ReturnAwaitModeJson>,
    /// Required Unicode regex flag for `require-unicode-regexp`.
    pub require_unicode_regexp_require_flag: Option<UnicodeRegexpRequireFlagJson>,
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
    /// Allow `rel="noreferrer"` without `noopener` in `no-blank-target`.
    pub no_blank_target_allow_no_referrer: Option<bool>,
    /// Domains allowed to use `target="_blank"` without rel hardening.
    pub no_blank_target_allow_domains: Option<Vec<String>>,
    /// Entropy threshold in tenths for `no-secrets`.
    pub no_secrets_entropy_threshold: Option<u32>,
    /// Allow named callbacks in `prefer-arrow-callback`.
    pub allow_named_functions_in_prefer_arrow_callback: Option<bool>,
    /// Allow unbound `this` in `prefer-arrow-callback`.
    pub allow_unbound_this_in_prefer_arrow_callback: Option<bool>,
    /// Allow `else if` chains in `no-else-return`.
    pub no_else_return_allow_else_if: Option<bool>,
    /// Allow keyword property names in `dot-notation`.
    pub dot_notation_allow_keywords: Option<bool>,
    /// Regex pattern of property names exempt from `dot-notation`.
    pub dot_notation_allow_pattern: Option<String>,
    /// Check nested boolean contexts in `no-extra-boolean-cast`.
    pub no_extra_boolean_cast_enforce_for_inner_expressions: Option<bool>,
    /// Enforcement mode for `eqeqeq`.
    pub eqeqeq_mode: Option<EqeqeqModeJson>,
    /// Ordering policy for `grouped-accessor-pairs`.
    pub grouped_accessor_pairs_order: Option<GroupedAccessorPairsOrderJson>,
    /// Enforce `grouped-accessor-pairs` in type-only member bodies.
    pub grouped_accessor_pairs_enforce_for_types: Option<bool>,
    /// Null comparison policy for `eqeqeq`.
    pub eqeqeq_null: Option<EqeqeqNullPolicyJson>,
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
    /// Ignore explicit `void` wrappers in `no-confusing-void-expression`.
    pub no_confusing_void_expression_ignore_void_operator: Option<bool>,
    /// Ignore returned void expressions inside void-returning functions.
    pub no_confusing_void_expression_ignore_void_returning_functions: Option<bool>,
    /// Check nested `var` declarations in `no-inner-declarations`.
    pub no_inner_declarations_check_var_declarations: Option<bool>,
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
    /// Allow explicit `void` to intentionally discard Promise results.
    pub allow_void_discard: Option<bool>,
    /// Allow explicit `void` returns in `no-promise-executor-return`.
    pub no_promise_executor_return_allow_void: Option<bool>,
    /// Check callback positions in `no-misused-promises`.
    pub check_misused_promises_in_callbacks: Option<bool>,
    /// Check conditionals in `no-misused-promises`.
    pub check_misused_promises_in_conditionals: Option<bool>,
    /// Check spread positions in `no-misused-promises`.
    pub check_misused_promises_in_spreads: Option<bool>,
    /// Check switch discriminants and cases in `use-isnan`.
    pub use_isnan_enforce_switch_case: Option<bool>,
    /// Check `indexOf` and `lastIndexOf` calls in `use-isnan`.
    pub use_isnan_enforce_index_of: Option<bool>,
    /// Check property assignments in `no-self-assign`.
    pub no_self_assign_check_properties: Option<bool>,
    /// Prefer top-level `import type` in `consistent-type-imports`.
    pub consistent_type_imports_prefer_type_imports: Option<bool>,
    /// Prefer inline `type` specifiers in `consistent-type-imports`.
    pub consistent_type_imports_prefer_inline_type_imports: Option<bool>,
    /// Disallow `import(\"...\")` type annotations in `consistent-type-imports`.
    pub consistent_type_imports_disallow_type_annotations: Option<bool>,
    /// Ignore conditional tests in `prefer-nullish-coalescing`.
    pub ignore_conditional_tests_in_prefer_nullish_coalescing: Option<bool>,
    /// Ignore mixed logical expressions in `prefer-nullish-coalescing`.
    pub ignore_mixed_logical_expressions_in_prefer_nullish_coalescing: Option<bool>,
    /// Ignore ternary checks in `prefer-nullish-coalescing`.
    pub ignore_ternary_tests_in_prefer_nullish_coalescing: Option<bool>,

    // restriction options
    /// Bitwise operators allowed by `no-bitwise`.
    pub allowed_bitwise_operators: Option<Vec<BitwiseOperatorJson>>,
    /// Allow `x | 0` int32 cast hints in `no-bitwise`.
    pub allow_bitwise_int32_hint: Option<bool>,
    /// Console methods allowed by `no-console`.
    pub allowed_console_methods: Option<Vec<String>>,
    /// Ignore explicit `any` in variadic parameter types for `no-explicit-any`.
    pub ignore_explicit_any_in_rest_args: Option<bool>,
    /// Allow labels on loop statements in `no-labels`.
    pub allow_loop_labels: Option<bool>,
    /// Allow labels on switch statements in `no-labels`.
    pub allow_switch_labels: Option<bool>,
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Option<Vec<f64>>,
    /// Check strict `=== null` and `!== null` comparisons in `no-null`.
    pub check_strict_null_equality: Option<bool>,
    /// Allow `declare namespace` and `declare module foo {}` in `no-namespace`.
    pub allow_namespace_declarations: Option<bool>,
    /// Allow namespace declarations in definition files for `no-namespace`.
    pub allow_namespace_definition_files: Option<bool>,
    /// Allow `++` and `--` in for-loop afterthoughts for `no-plusplus`.
    pub allow_plusplus_for_loop_afterthoughts: Option<bool>,
    /// Allow static require target patterns for `no-require-imports`.
    pub allowed_require_import_patterns: Option<Vec<String>>,
    /// Allow `import foo = require("foo")` for `no-require-imports`.
    pub allow_require_import_aliases: Option<bool>,
    /// Where `no-warning-comments` should match terms.
    pub warning_comment_location: Option<WarningCommentLocationJson>,
    /// Decoration characters to ignore at the start of `no-warning-comments`.
    pub warning_comment_decoration: Option<Vec<String>>,
    /// Globals to restrict.
    pub restricted_globals: Option<Vec<String>>,
    /// Import paths to restrict.
    pub restricted_imports: Option<Vec<String>>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Option<Vec<String>>,
    /// Module boundary constraints for module boundary aware lint rules.
    #[serde(alias = "architecture")]
    pub module_boundaries: Option<LinterModuleBoundariesJson>,
}

impl LinterJson {
    /// Validate configuration values that need semantic checking.
    pub fn validate(&self) -> Result<(), String> {
        validate_regex_patterns(
            "linter.allowedRequireImportPatterns",
            self.allowed_require_import_patterns.as_deref(),
        )?;
        validate_regex_patterns(
            "linter.noFallthroughCommentPattern",
            self.no_fallthrough_comment_pattern
                .as_ref()
                .map(std::slice::from_ref),
        )?;
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
        validate_single_character_strings(
            "linter.noUselessEscapeAllowRegexCharacters",
            self.no_useless_escape_allow_regex_characters.as_deref(),
        )?;

        Ok(())
    }

    /// Apply linter options to a LinterOptions struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }
        self.rules.apply(options);

        // correctness options
        if let Some(allow_void_discard) = self.allow_void_discard {
            options.allow_void_discard = allow_void_discard;
        }
        if let Some(no_promise_executor_return_allow_void) =
            self.no_promise_executor_return_allow_void
        {
            options.no_promise_executor_return_allow_void = no_promise_executor_return_allow_void;
        }
        if let Some(check_misused_promises_in_callbacks) = self.check_misused_promises_in_callbacks
        {
            options.check_misused_promises_in_callbacks = check_misused_promises_in_callbacks;
        }
        if let Some(check_misused_promises_in_conditionals) =
            self.check_misused_promises_in_conditionals
        {
            options.check_misused_promises_in_conditionals = check_misused_promises_in_conditionals;
        }
        if let Some(check_misused_promises_in_spreads) = self.check_misused_promises_in_spreads {
            options.check_misused_promises_in_spreads = check_misused_promises_in_spreads;
        }
        if let Some(use_isnan_enforce_switch_case) = self.use_isnan_enforce_switch_case {
            options.use_isnan_enforce_switch_case = use_isnan_enforce_switch_case;
        }
        if let Some(use_isnan_enforce_index_of) = self.use_isnan_enforce_index_of {
            options.use_isnan_enforce_index_of = use_isnan_enforce_index_of;
        }
        if let Some(no_self_assign_check_properties) = self.no_self_assign_check_properties {
            options.no_self_assign_check_properties = no_self_assign_check_properties;
        }

        // complexity thresholds
        if let Some(max_booleans) = self.max_booleans {
            options.max_booleans = max_booleans;
        }
        if let Some(max_cognitive_complexity) = self.max_cognitive_complexity {
            options.max_cognitive_complexity = max_cognitive_complexity;
        }
        if let Some(max_cyclomatic_complexity) = self.max_cyclomatic_complexity {
            options.max_cyclomatic_complexity = max_cyclomatic_complexity;
        }
        if let Some(cyclomatic_complexity_variant) = self.cyclomatic_complexity_variant {
            options.cyclomatic_complexity_variant = cyclomatic_complexity_variant.into();
        }
        if let Some(max_depth) = self.max_depth {
            options.max_depth = max_depth;
        }
        if let Some(max_lines) = self.max_lines {
            options.max_lines = max_lines;
        }
        if let Some(max_lines_skip_comments) = self.max_lines_skip_comments {
            options.max_lines_skip_comments = max_lines_skip_comments;
        }
        if let Some(max_lines_skip_blank_lines) = self.max_lines_skip_blank_lines {
            options.max_lines_skip_blank_lines = max_lines_skip_blank_lines;
        }
        if let Some(max_lines_per_function) = self.max_lines_per_function {
            options.max_lines_per_function = max_lines_per_function;
        }
        if let Some(max_lines_per_function_skip_comments) =
            self.max_lines_per_function_skip_comments
        {
            options.max_lines_per_function_skip_comments = max_lines_per_function_skip_comments;
        }
        if let Some(max_lines_per_function_skip_blank_lines) =
            self.max_lines_per_function_skip_blank_lines
        {
            options.max_lines_per_function_skip_blank_lines =
                max_lines_per_function_skip_blank_lines;
        }
        if let Some(max_lines_per_function_include_iifes) =
            self.max_lines_per_function_include_iifes
        {
            options.max_lines_per_function_include_iifes = max_lines_per_function_include_iifes;
        }
        if let Some(max_nested_callbacks) = self.max_nested_callbacks {
            options.max_nested_callbacks = max_nested_callbacks;
        }
        if let Some(max_params) = self.max_params {
            options.max_params = max_params;
        }
        if let Some(max_params_count_this) = self.max_params_count_this {
            options.max_params_count_this = max_params_count_this.into();
        } else if let Some(max_params_count_void_this) = self.max_params_count_void_this {
            options.max_params_count_this = if max_params_count_void_this {
                MaxParamsCountThis::Always
            } else {
                MaxParamsCountThis::ExceptVoid
            };
        }
        if let Some(max_statements) = self.max_statements {
            options.max_statements = max_statements;
        }
        if let Some(max_statements_ignore_top_level_functions) =
            self.max_statements_ignore_top_level_functions
        {
            options.max_statements_ignore_top_level_functions =
                max_statements_ignore_top_level_functions;
        }
        if let Some(max_return_statements) = self.max_return_statements {
            options.max_return_statements = max_return_statements;
        }
        if let Some(max_switch_cases) = self.max_switch_cases {
            options.max_switch_cases = max_switch_cases;
        }
        if let Some(max_type_variants) = self.max_type_variants {
            options.max_type_variants = max_type_variants;
        }
        if let Some(max_type_fields) = self.max_type_fields {
            options.max_type_fields = max_type_fields;
        }
        if let Some(max_type_complexity) = self.max_type_complexity {
            options.max_type_complexity = max_type_complexity;
        }
        if let Some(max_duplicate_string_occurrences) = self.max_duplicate_string_occurrences {
            options.max_duplicate_string_occurrences = max_duplicate_string_occurrences;
        }
        if let Some(max_try_block_statements) = self.max_try_block_statements {
            options.max_try_block_statements = max_try_block_statements;
        }
        if let Some(no_multi_assign_ignore_non_declaration) =
            self.no_multi_assign_ignore_non_declaration
        {
            options.no_multi_assign_ignore_non_declaration = no_multi_assign_ignore_non_declaration;
        }
        if let Some(no_unused_expressions_allow_short_circuit) =
            self.no_unused_expressions_allow_short_circuit
        {
            options.no_unused_expressions_allow_short_circuit =
                no_unused_expressions_allow_short_circuit;
        }
        if let Some(no_unused_expressions_allow_ternary) = self.no_unused_expressions_allow_ternary
        {
            options.no_unused_expressions_allow_ternary = no_unused_expressions_allow_ternary;
        }
        if let Some(no_unused_expressions_allow_tagged_templates) =
            self.no_unused_expressions_allow_tagged_templates
        {
            options.no_unused_expressions_allow_tagged_templates =
                no_unused_expressions_allow_tagged_templates;
        }
        if let Some(no_unused_expressions_enforce_for_jsx) =
            self.no_unused_expressions_enforce_for_jsx
        {
            options.no_unused_expressions_enforce_for_jsx = no_unused_expressions_enforce_for_jsx;
        }
        if let Some(no_unused_expressions_ignore_directives) =
            self.no_unused_expressions_ignore_directives
        {
            options.no_unused_expressions_ignore_directives =
                no_unused_expressions_ignore_directives;
        }

        // style options
        if let Some(array_type) = self.array_type {
            options.array_type = array_type.into();
        }
        if let Some(type_definition_style) = self.type_definition_style {
            options.type_definition_style = type_definition_style.into();
        }
        if let Some(ref catch_error_name) = self.catch_error_name {
            options.catch_error_name = catch_error_name.clone();
        }
        if let Some(filename_case) = self.filename_case {
            options.filename_case = filename_case.into();
        }
        if let Some(allow_single_extends_empty_interface) =
            self.allow_single_extends_empty_interface
        {
            options.allow_single_extends_empty_interface = allow_single_extends_empty_interface;
        }
        if let Some(return_await_mode) = self.return_await_mode {
            options.return_await_mode = return_await_mode.into();
        }
        if let Some(require_unicode_regexp_require_flag) = self.require_unicode_regexp_require_flag
        {
            options.require_unicode_regexp_require_flag =
                require_unicode_regexp_require_flag.into();
        }
        if let Some(allow_named_functions_in_prefer_arrow_callback) =
            self.allow_named_functions_in_prefer_arrow_callback
        {
            options.allow_named_functions_in_prefer_arrow_callback =
                allow_named_functions_in_prefer_arrow_callback;
        }
        if let Some(allow_unbound_this_in_prefer_arrow_callback) =
            self.allow_unbound_this_in_prefer_arrow_callback
        {
            options.allow_unbound_this_in_prefer_arrow_callback =
                allow_unbound_this_in_prefer_arrow_callback;
        }
        if let Some(no_else_return_allow_else_if) = self.no_else_return_allow_else_if {
            options.no_else_return_allow_else_if = no_else_return_allow_else_if;
        }
        if let Some(dot_notation_allow_keywords) = self.dot_notation_allow_keywords {
            options.dot_notation_allow_keywords = dot_notation_allow_keywords;
        }
        if let Some(ref dot_notation_allow_pattern) = self.dot_notation_allow_pattern {
            options.dot_notation_allow_pattern = Some(dot_notation_allow_pattern.clone());
        }
        if let Some(no_extra_boolean_cast_enforce_for_inner_expressions) =
            self.no_extra_boolean_cast_enforce_for_inner_expressions
        {
            options.no_extra_boolean_cast_enforce_for_inner_expressions =
                no_extra_boolean_cast_enforce_for_inner_expressions;
        }
        if let Some(eqeqeq_mode) = self.eqeqeq_mode {
            options.eqeqeq_mode = eqeqeq_mode.into();
        }
        if let Some(grouped_accessor_pairs_order) = self.grouped_accessor_pairs_order {
            options.grouped_accessor_pairs_order = grouped_accessor_pairs_order.into();
        }
        if let Some(grouped_accessor_pairs_enforce_for_types) =
            self.grouped_accessor_pairs_enforce_for_types
        {
            options.grouped_accessor_pairs_enforce_for_types =
                grouped_accessor_pairs_enforce_for_types;
        }
        if let Some(eqeqeq_null) = self.eqeqeq_null {
            options.eqeqeq_null = eqeqeq_null.into();
        }
        if let Some(operator_assignment_mode) = self.operator_assignment_mode {
            options.operator_assignment_mode = operator_assignment_mode.into();
        }
        if let Some(object_shorthand_mode) = self.object_shorthand_mode {
            options.object_shorthand_mode = object_shorthand_mode.into();
        }
        if let Some(object_shorthand_avoid_quotes) = self.object_shorthand_avoid_quotes {
            options.object_shorthand_avoid_quotes = object_shorthand_avoid_quotes;
        }
        if let Some(object_shorthand_ignore_constructors) =
            self.object_shorthand_ignore_constructors
        {
            options.object_shorthand_ignore_constructors = object_shorthand_ignore_constructors;
        }
        if let Some(ref object_shorthand_methods_ignore_pattern) =
            self.object_shorthand_methods_ignore_pattern
        {
            options.object_shorthand_methods_ignore_pattern =
                Some(object_shorthand_methods_ignore_pattern.clone());
        }
        if let Some(object_shorthand_avoid_explicit_return_arrows) =
            self.object_shorthand_avoid_explicit_return_arrows
        {
            options.object_shorthand_avoid_explicit_return_arrows =
                object_shorthand_avoid_explicit_return_arrows;
        }
        if let Some(no_unneeded_ternary_default_assignment) =
            self.no_unneeded_ternary_default_assignment
        {
            options.no_unneeded_ternary_default_assignment = no_unneeded_ternary_default_assignment;
        }
        if let Some(prefer_promise_reject_errors_allow_empty_reject) =
            self.prefer_promise_reject_errors_allow_empty_reject
        {
            options.prefer_promise_reject_errors_allow_empty_reject =
                prefer_promise_reject_errors_allow_empty_reject;
        }
        if let Some(prefer_const_destructuring) = self.prefer_const_destructuring {
            options.prefer_const_destructuring = prefer_const_destructuring.into();
        }
        if let Some(prefer_const_ignore_read_before_assign) =
            self.prefer_const_ignore_read_before_assign
        {
            options.prefer_const_ignore_read_before_assign = prefer_const_ignore_read_before_assign;
        }
        if let Some(yoda_mode) = self.yoda_mode {
            options.yoda_mode = yoda_mode.into();
        }
        if let Some(yoda_except_range) = self.yoda_except_range {
            options.yoda_except_range = yoda_except_range;
        }
        if let Some(yoda_only_equality) = self.yoda_only_equality {
            options.yoda_only_equality = yoda_only_equality;
        }
        if let Some(sort_imports_ignore_case) = self.sort_imports_ignore_case {
            options.sort_imports_ignore_case = sort_imports_ignore_case;
        }
        if let Some(sort_imports_ignore_declaration_sort) =
            self.sort_imports_ignore_declaration_sort
        {
            options.sort_imports_ignore_declaration_sort = sort_imports_ignore_declaration_sort;
        }
        if let Some(sort_imports_ignore_member_sort) = self.sort_imports_ignore_member_sort {
            options.sort_imports_ignore_member_sort = sort_imports_ignore_member_sort;
        }
        if let Some(sort_imports_allow_separated_groups) = self.sort_imports_allow_separated_groups
        {
            options.sort_imports_allow_separated_groups = sort_imports_allow_separated_groups;
        }
        if let Some(ref sort_imports_member_syntax_sort_order) =
            self.sort_imports_member_syntax_sort_order
        {
            options.sort_imports_member_syntax_sort_order = sort_imports_member_syntax_sort_order
                .iter()
                .copied()
                .map(Into::into)
                .collect();
        }
        if let Some(no_confusing_void_expression_ignore_void_operator) =
            self.no_confusing_void_expression_ignore_void_operator
        {
            options.no_confusing_void_expression_ignore_void_operator =
                no_confusing_void_expression_ignore_void_operator;
        }
        if let Some(no_confusing_void_expression_ignore_void_returning_functions) =
            self.no_confusing_void_expression_ignore_void_returning_functions
        {
            options.no_confusing_void_expression_ignore_void_returning_functions =
                no_confusing_void_expression_ignore_void_returning_functions;
        }
        if let Some(no_inner_declarations_check_var_declarations) =
            self.no_inner_declarations_check_var_declarations
        {
            options.no_inner_declarations_check_var_declarations =
                no_inner_declarations_check_var_declarations;
        }
        if let Some(no_cond_assign_mode) = self.no_cond_assign_mode {
            options.no_cond_assign_mode = no_cond_assign_mode.into();
        }
        if let Some(no_empty_allow_empty_catch) = self.no_empty_allow_empty_catch {
            options.no_empty_allow_empty_catch = no_empty_allow_empty_catch;
        }
        if let Some(no_empty_pattern_allow_object_patterns_as_parameters) =
            self.no_empty_pattern_allow_object_patterns_as_parameters
        {
            options.no_empty_pattern_allow_object_patterns_as_parameters =
                no_empty_pattern_allow_object_patterns_as_parameters;
        }
        if let Some(ref no_empty_function_allow) = self.no_empty_function_allow {
            options.no_empty_function_allow = no_empty_function_allow
                .iter()
                .copied()
                .map(Into::into)
                .collect();
        }
        if let Some(no_fallthrough_allow_empty_case) = self.no_fallthrough_allow_empty_case {
            options.no_fallthrough_allow_empty_case = no_fallthrough_allow_empty_case;
        }
        if let Some(ref no_fallthrough_comment_pattern) = self.no_fallthrough_comment_pattern {
            options.no_fallthrough_comment_pattern = Some(no_fallthrough_comment_pattern.clone());
        }
        if let Some(no_fallthrough_report_unused_comment) =
            self.no_fallthrough_report_unused_comment
        {
            options.no_fallthrough_report_unused_comment = no_fallthrough_report_unused_comment;
        }
        if let Some(ref ignored_unused_parameter_prefixes) = self.ignored_unused_parameter_prefixes
        {
            options.ignored_unused_parameter_prefixes = ignored_unused_parameter_prefixes.clone();
        }
        if let Some(no_useless_rename_ignore_destructuring) =
            self.no_useless_rename_ignore_destructuring
        {
            options.no_useless_rename_ignore_destructuring = no_useless_rename_ignore_destructuring;
        }
        if let Some(no_useless_rename_ignore_import) = self.no_useless_rename_ignore_import {
            options.no_useless_rename_ignore_import = no_useless_rename_ignore_import;
        }
        if let Some(no_useless_rename_ignore_export) = self.no_useless_rename_ignore_export {
            options.no_useless_rename_ignore_export = no_useless_rename_ignore_export;
        }
        if let Some(ref no_useless_escape_allow_regex_characters) =
            self.no_useless_escape_allow_regex_characters
        {
            options.no_useless_escape_allow_regex_characters =
                no_useless_escape_allow_regex_characters.clone();
        }
        if let Some(no_blank_target_allow_no_referrer) = self.no_blank_target_allow_no_referrer {
            options.no_blank_target_allow_no_referrer = no_blank_target_allow_no_referrer;
        }
        if let Some(ref no_blank_target_allow_domains) = self.no_blank_target_allow_domains {
            options.no_blank_target_allow_domains = no_blank_target_allow_domains.clone();
        }
        if let Some(no_secrets_entropy_threshold) = self.no_secrets_entropy_threshold {
            options.no_secrets_entropy_threshold = no_secrets_entropy_threshold;
        }
        if let Some(consistent_type_imports_prefer_type_imports) =
            self.consistent_type_imports_prefer_type_imports
        {
            options.consistent_type_imports_prefer_type_imports =
                consistent_type_imports_prefer_type_imports;
        }
        if let Some(consistent_type_imports_prefer_inline_type_imports) =
            self.consistent_type_imports_prefer_inline_type_imports
        {
            options.consistent_type_imports_prefer_inline_type_imports =
                consistent_type_imports_prefer_inline_type_imports;
        }
        if let Some(consistent_type_imports_disallow_type_annotations) =
            self.consistent_type_imports_disallow_type_annotations
        {
            options.consistent_type_imports_disallow_type_annotations =
                consistent_type_imports_disallow_type_annotations;
        }
        if let Some(ignore_conditional_tests_in_prefer_nullish_coalescing) =
            self.ignore_conditional_tests_in_prefer_nullish_coalescing
        {
            options.ignore_conditional_tests_in_prefer_nullish_coalescing =
                ignore_conditional_tests_in_prefer_nullish_coalescing;
        }
        if let Some(ignore_mixed_logical_expressions_in_prefer_nullish_coalescing) =
            self.ignore_mixed_logical_expressions_in_prefer_nullish_coalescing
        {
            options.ignore_mixed_logical_expressions_in_prefer_nullish_coalescing =
                ignore_mixed_logical_expressions_in_prefer_nullish_coalescing;
        }
        if let Some(ignore_ternary_tests_in_prefer_nullish_coalescing) =
            self.ignore_ternary_tests_in_prefer_nullish_coalescing
        {
            options.ignore_ternary_tests_in_prefer_nullish_coalescing =
                ignore_ternary_tests_in_prefer_nullish_coalescing;
        }

        // restriction options
        if let Some(ref allowed_bitwise_operators) = self.allowed_bitwise_operators {
            options.allowed_bitwise_operators = allowed_bitwise_operators
                .iter()
                .copied()
                .map(Into::into)
                .collect();
        }
        if let Some(allow_bitwise_int32_hint) = self.allow_bitwise_int32_hint {
            options.allow_bitwise_int32_hint = allow_bitwise_int32_hint;
        }
        if let Some(ref allowed_console_methods) = self.allowed_console_methods {
            options.allowed_console_methods = allowed_console_methods.clone();
        }
        if let Some(ignore_explicit_any_in_rest_args) = self.ignore_explicit_any_in_rest_args {
            options.ignore_explicit_any_in_rest_args = ignore_explicit_any_in_rest_args;
        }
        if let Some(allow_loop_labels) = self.allow_loop_labels {
            options.allow_loop_labels = allow_loop_labels;
        }
        if let Some(allow_switch_labels) = self.allow_switch_labels {
            options.allow_switch_labels = allow_switch_labels;
        }
        if let Some(ref allowed_magic_numbers) = self.allowed_magic_numbers {
            options.allowed_magic_numbers = allowed_magic_numbers.clone();
        }
        if let Some(check_strict_null_equality) = self.check_strict_null_equality {
            options.check_strict_null_equality = check_strict_null_equality;
        }
        if let Some(allow_namespace_declarations) = self.allow_namespace_declarations {
            options.allow_namespace_declarations = allow_namespace_declarations;
        }
        if let Some(allow_namespace_definition_files) = self.allow_namespace_definition_files {
            options.allow_namespace_definition_files = allow_namespace_definition_files;
        }
        if let Some(allow_plusplus_for_loop_afterthoughts) =
            self.allow_plusplus_for_loop_afterthoughts
        {
            options.allow_plusplus_for_loop_afterthoughts = allow_plusplus_for_loop_afterthoughts;
        }
        if let Some(ref allowed_require_import_patterns) = self.allowed_require_import_patterns {
            options.allowed_require_import_patterns = allowed_require_import_patterns.clone();
        }
        if let Some(allow_require_import_aliases) = self.allow_require_import_aliases {
            options.allow_require_import_aliases = allow_require_import_aliases;
        }
        if let Some(warning_comment_location) = self.warning_comment_location {
            options.warning_comment_location = warning_comment_location.into();
        }
        if let Some(ref warning_comment_decoration) = self.warning_comment_decoration {
            options.warning_comment_decoration = warning_comment_decoration.clone();
        }
        if let Some(ref restricted_globals) = self.restricted_globals {
            options.restricted_globals = restricted_globals.clone();
        }
        if let Some(ref restricted_imports) = self.restricted_imports {
            options.restricted_imports = restricted_imports.clone();
        }
        if let Some(ref warning_comment_terms) = self.warning_comment_terms {
            options.warning_comment_terms = warning_comment_terms.clone();
        }

        // module boundary options
        if let Some(ref module_boundaries) = self.module_boundaries {
            module_boundaries.apply(&mut options.module_boundaries);
        }
    }
}

/// Cached compiled regex sets for `allowedRequireImportPatterns`.
static ALLOWED_REQUIRE_IMPORT_PATTERNS_CACHE: LazyLock<Mutex<HashMap<Vec<String>, Arc<[Regex]>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Cached compiled regex patterns for `noFallthroughCommentPattern`.
static NO_FALLTHROUGH_COMMENT_PATTERN_CACHE: LazyLock<Mutex<HashMap<String, Arc<Regex>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Validate one list of regex patterns.
fn validate_regex_patterns(field_name: &str, patterns: Option<&[String]>) -> Result<(), String> {
    let Some(patterns) = patterns else {
        return Ok(());
    };

    for pattern in patterns {
        Regex::new(pattern)
            .map_err(|error| format!("invalid regex in {field_name}: `{pattern}`: {error}"))?;
    }

    Ok(())
}

/// Validate one list of single-character strings.
fn validate_single_character_strings(
    field_name: &str,
    values: Option<&[String]>,
) -> Result<(), String> {
    let Some(values) = values else {
        return Ok(());
    };

    for value in values {
        if value.chars().count() != 1 {
            return Err(format!(
                "invalid value in {field_name}: `{value}` must be exactly one character"
            ));
        }
    }

    Ok(())
}

/// Validate that one enum list is a permutation of the expected values.
fn validate_exact_enum_order<T: PartialEq + Copy>(
    field_name: &str,
    values: Option<&[T]>,
    expected_values: &[T],
) -> Result<(), String> {
    let Some(values) = values else {
        return Ok(());
    };

    if values.len() != expected_values.len() {
        return Err(format!(
            "{field_name} must contain exactly {} values",
            expected_values.len()
        ));
    }

    for expected_value in expected_values {
        let occurrences = values
            .iter()
            .copied()
            .filter(|value| value == expected_value)
            .count();
        if occurrences != 1 {
            return Err(format!(
                "{field_name} must contain each member syntax value exactly once"
            ));
        }
    }

    Ok(())
}

/// Return compiled allow patterns for `no-require-imports`.
pub fn compiled_allowed_require_import_patterns(patterns: &[String]) -> Arc<[Regex]> {
    let mut cache = ALLOWED_REQUIRE_IMPORT_PATTERNS_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(compiled_patterns) = cache.get(patterns) {
        return compiled_patterns.clone();
    }

    // validated config invariant
    let compiled_patterns = patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern)
                .expect("validated config invariant: no-require-imports allow patterns compile")
        })
        .collect::<Vec<_>>();
    let compiled_patterns = Arc::<[Regex]>::from(compiled_patterns);
    cache.insert(patterns.to_vec(), compiled_patterns.clone());
    compiled_patterns
}

/// Return a compiled comment pattern for `no-fallthrough`.
pub fn compiled_no_fallthrough_comment_pattern(pattern: Option<&str>) -> Option<Arc<Regex>> {
    let pattern = pattern?;

    let mut cache = NO_FALLTHROUGH_COMMENT_PATTERN_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(compiled_pattern) = cache.get(pattern) {
        return Some(compiled_pattern.clone());
    }

    // validated config invariant
    let compiled_pattern = Arc::new(
        Regex::new(pattern)
            .expect("validated config invariant: no-fallthrough comment pattern compiles"),
    );
    cache.insert(pattern.to_string(), compiled_pattern.clone());
    Some(compiled_pattern)
}

#[cfg(test)]
mod tests {
    use super::LinterJson;

    #[test]
    fn test_rejects_invalid_no_useless_escape_allow_regex_characters() {
        let json = LinterJson {
            no_useless_escape_allow_regex_characters: Some(vec!["ab".to_string()]),
            ..Default::default()
        };

        let error = json
            .validate()
            .expect_err("expected invalid character allowlist");
        assert!(
            error.contains("noUselessEscapeAllowRegexCharacters"),
            "unexpected error: {error}"
        );
    }
}

/// Module boundary options for linter configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleBoundariesJson {
    /// Policy for modules that do not match any configured component.
    pub unknown_component_policy: Option<DiagnosticPolicyJson>,
    /// Declared module components.
    pub components: Option<Vec<LinterModuleComponentJson>>,
    /// Allowed component to component dependency rules.
    pub rules: Option<Vec<LinterModuleDependencyRuleJson>>,
    /// Explicit dependency exceptions.
    pub exceptions: Option<Vec<LinterModuleDependencyExceptionJson>>,
}

impl LinterModuleBoundariesJson {
    /// Apply module boundary options to one linter module boundary options struct.
    pub fn apply(&self, options: &mut LintModuleBoundariesOptions) {
        if let Some(unknown_component_policy) = self.unknown_component_policy {
            options.unknown_component_policy = unknown_component_policy.into();
        }
        if let Some(ref components) = self.components {
            options.components = components.iter().map(LintModuleComponent::from).collect();
        }
        if let Some(ref rules) = self.rules {
            options.dependency_rules = rules.iter().map(LintModuleDependencyRule::from).collect();
        }
        if let Some(ref exceptions) = self.exceptions {
            options.exceptions = exceptions
                .iter()
                .map(LintModuleDependencyException::from)
                .collect();
        }
    }
}

/// One module component declaration in linter JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleComponentJson {
    /// The unique component name.
    pub name: String,
    /// Glob patterns used to map modules into this component.
    #[serde(default, rename = "match")]
    pub path_patterns: Vec<String>,
}

impl From<&LinterModuleComponentJson> for LintModuleComponent {
    fn from(value: &LinterModuleComponentJson) -> Self {
        Self {
            name: value.name.clone(),
            path_patterns: value.path_patterns.clone(),
        }
    }
}

/// One module dependency rule in linter JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleDependencyRuleJson {
    /// The source component name.
    pub from: String,
    /// The list of allowed destination component names.
    #[serde(default)]
    pub allow: Vec<String>,
}

impl From<&LinterModuleDependencyRuleJson> for LintModuleDependencyRule {
    fn from(value: &LinterModuleDependencyRuleJson) -> Self {
        Self {
            from: value.from.clone(),
            allow: value.allow.clone(),
        }
    }
}

/// One module dependency exception in linter JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleDependencyExceptionJson {
    /// The source component name.
    pub from: String,
    /// The destination component name.
    pub to: String,
    /// Module path patterns where this exception is allowed.
    #[serde(default, rename = "match")]
    pub path_patterns: Vec<String>,
    /// Optional human-readable reason for the exception.
    pub reason: Option<String>,
}

impl From<&LinterModuleDependencyExceptionJson> for LintModuleDependencyException {
    fn from(value: &LinterModuleDependencyExceptionJson) -> Self {
        Self {
            from: value.from.clone(),
            to: value.to.clone(),
            path_patterns: value.path_patterns.clone(),
            reason: value.reason.clone(),
        }
    }
}

/// Linter rules configuration.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterRulesJson {
    /// Preset: "none", "recommended", "strict", or "all".
    pub preset: Option<String>,
    /// Enable the recommended rule set (shorthand for preset: "recommended").
    pub recommended: Option<bool>,
    /// Enable all rules (shorthand for preset: "all").
    pub all: Option<bool>,
    /// Category-level severity overrides.
    pub categories: Option<IndexMap<LintCategoryJson, RuleSeverityJson>>,
    /// Individual rule overrides (rule name -> severity).
    #[serde(flatten)]
    pub overrides: IndexMap<String, RuleSeverityJson>,
}

impl LinterRulesJson {
    /// Apply rules configuration to LinterOptions.
    pub fn apply(&self, options: &mut LinterOptions) {
        // preset field takes precedence
        if let Some(preset_str) = &self.preset {
            if let Some(preset) = LintPreset::parse(preset_str) {
                options.preset = preset;
            }
        } else if let Some(true) = self.all {
            options.preset = LintPreset::All;
        } else if let Some(recommended) = self.recommended {
            options.preset = if recommended {
                LintPreset::Recommended
            } else {
                LintPreset::None
            };
        }

        // category overrides
        if let Some(categories) = &self.categories {
            for (category, severity) in categories {
                options
                    .categories
                    .insert((*category).into(), (*severity).into());
            }
        }

        // rule overrides
        for (rule, severity) in &self.overrides {
            options.overrides.insert(rule.clone(), (*severity).into());
        }
    }
}

/// Rule severity for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverityJson {
    /// Rule is disabled.
    Off,
    /// Rule produces warnings.
    Warn,
    /// Rule produces errors.
    Error,
}

impl From<RuleSeverityJson> for LintSeverity {
    fn from(value: RuleSeverityJson) -> Self {
        match value {
            RuleSeverityJson::Off => LintSeverity::Off,
            RuleSeverityJson::Warn => LintSeverity::Warning,
            RuleSeverityJson::Error => LintSeverity::Error,
        }
    }
}

/// Lint category for JSON deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LintCategoryJson {
    /// Correctness lints detect likely bugs and logic errors.
    Correctness,
    /// Suspicious lints detect code that is likely unintentional.
    Suspicious,
    /// Performance lints detect inefficient patterns.
    Performance,
    /// Style lints enforce consistent coding style.
    Style,
    /// Security lints detect potential vulnerabilities.
    Security,
    /// Complexity lints detect overly complex code.
    Complexity,
    /// Restriction lints enforce project-specific restrictions.
    Restriction,
}

impl From<LintCategoryJson> for LintCategory {
    fn from(value: LintCategoryJson) -> Self {
        match value {
            LintCategoryJson::Correctness => LintCategory::Correctness,
            LintCategoryJson::Suspicious => LintCategory::Suspicious,
            LintCategoryJson::Performance => LintCategory::Performance,
            LintCategoryJson::Style => LintCategory::Style,
            LintCategoryJson::Security => LintCategory::Security,
            LintCategoryJson::Complexity => LintCategory::Complexity,
            LintCategoryJson::Restriction => LintCategory::Restriction,
        }
    }
}

/// Preferred array type syntax for the `array-type` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ArrayTypeStyleJson {
    /// Prefer `T[]` syntax.
    Array,
    /// Prefer `Array<T>` syntax.
    Generic,
}

impl From<ArrayTypeStyleJson> for ArrayTypeStyle {
    fn from(value: ArrayTypeStyleJson) -> Self {
        match value {
            ArrayTypeStyleJson::Array => ArrayTypeStyle::Array,
            ArrayTypeStyleJson::Generic => ArrayTypeStyle::Generic,
        }
    }
}

/// Preferred type definition syntax for the `consistent-type-definitions` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TypeDefinitionStyleJson {
    /// Prefer `type` aliases.
    Type,
    /// Prefer `interface` declarations.
    Interface,
}

impl From<TypeDefinitionStyleJson> for TypeDefinitionStyle {
    fn from(value: TypeDefinitionStyleJson) -> Self {
        match value {
            TypeDefinitionStyleJson::Type => TypeDefinitionStyle::Type,
            TypeDefinitionStyleJson::Interface => TypeDefinitionStyle::Interface,
        }
    }
}

/// Filename case style for the `filename-case` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FilenameCaseJson {
    /// kebab-case (e.g., `my-component.ts`).
    Kebab,
    /// snake_case (e.g., `my_component.ts`).
    Snake,
    /// camelCase (e.g., `myComponent.ts`).
    Camel,
    /// PascalCase (e.g., `MyComponent.ts`).
    Pascal,
}

impl From<FilenameCaseJson> for FilenameCase {
    fn from(value: FilenameCaseJson) -> Self {
        match value {
            FilenameCaseJson::Kebab => FilenameCase::Kebab,
            FilenameCaseJson::Snake => FilenameCase::Snake,
            FilenameCaseJson::Camel => FilenameCase::Camel,
            FilenameCaseJson::Pascal => FilenameCase::Pascal,
        }
    }
}

/// Return-await mode for linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ReturnAwaitModeJson {
    /// Require await only in error handling contexts and forbid it elsewhere.
    InTryCatch,
    /// Require await only in error handling contexts and do not enforce elsewhere.
    ErrorHandlingCorrectnessOnly,
    /// Require await in all contexts.
    Always,
    /// Forbid await in all contexts.
    Never,
}

impl From<ReturnAwaitModeJson> for ReturnAwaitMode {
    fn from(value: ReturnAwaitModeJson) -> Self {
        match value {
            ReturnAwaitModeJson::InTryCatch => ReturnAwaitMode::InTryCatch,
            ReturnAwaitModeJson::ErrorHandlingCorrectnessOnly => {
                ReturnAwaitMode::ErrorHandlingCorrectnessOnly
            }
            ReturnAwaitModeJson::Always => ReturnAwaitMode::Always,
            ReturnAwaitModeJson::Never => ReturnAwaitMode::Never,
        }
    }
}

/// Required Unicode regex flag accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum UnicodeRegexpRequireFlagJson {
    /// Require the `u` flag.
    U,
    /// Require the `v` flag.
    V,
}

impl From<UnicodeRegexpRequireFlagJson> for UnicodeRegexpRequireFlag {
    fn from(value: UnicodeRegexpRequireFlagJson) -> Self {
        match value {
            UnicodeRegexpRequireFlagJson::U => UnicodeRegexpRequireFlag::U,
            UnicodeRegexpRequireFlagJson::V => UnicodeRegexpRequireFlag::V,
        }
    }
}

/// Cyclomatic-complexity variant accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CyclomaticComplexityVariantJson {
    /// Count each non-default switch case.
    Classic,
    /// Count each switch once regardless of case count.
    Modified,
}

impl From<CyclomaticComplexityVariantJson> for CyclomaticComplexityVariant {
    fn from(value: CyclomaticComplexityVariantJson) -> Self {
        match value {
            CyclomaticComplexityVariantJson::Classic => CyclomaticComplexityVariant::Classic,
            CyclomaticComplexityVariantJson::Modified => CyclomaticComplexityVariant::Modified,
        }
    }
}

/// Max-params `this` counting mode accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MaxParamsCountThisJson {
    /// Never count `this`.
    Never,
    /// Count `this` unless it is explicitly typed as `void`.
    ExceptVoid,
    /// Always count `this`.
    Always,
}

impl From<MaxParamsCountThisJson> for MaxParamsCountThis {
    fn from(value: MaxParamsCountThisJson) -> Self {
        match value {
            MaxParamsCountThisJson::Never => MaxParamsCountThis::Never,
            MaxParamsCountThisJson::ExceptVoid => MaxParamsCountThis::ExceptVoid,
            MaxParamsCountThisJson::Always => MaxParamsCountThis::Always,
        }
    }
}

/// Eqeqeq enforcement mode accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EqeqeqModeJson {
    /// Always require strict equality operators.
    Always,
    /// Allow loose equality in the upstream smart cases.
    Smart,
    /// Allow loose equality only for null checks.
    AllowNull,
}

impl From<EqeqeqModeJson> for EqeqeqMode {
    fn from(value: EqeqeqModeJson) -> Self {
        match value {
            EqeqeqModeJson::Always => EqeqeqMode::Always,
            EqeqeqModeJson::Smart => EqeqeqMode::Smart,
            EqeqeqModeJson::AllowNull => EqeqeqMode::AllowNull,
        }
    }
}

/// Eqeqeq null policy accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EqeqeqNullPolicyJson {
    /// Require strict null equality.
    Always,
    /// Require loose null equality.
    Never,
    /// Ignore null equality style.
    Ignore,
}

impl From<EqeqeqNullPolicyJson> for EqeqeqNullPolicy {
    fn from(value: EqeqeqNullPolicyJson) -> Self {
        match value {
            EqeqeqNullPolicyJson::Always => EqeqeqNullPolicy::Always,
            EqeqeqNullPolicyJson::Never => EqeqeqNullPolicy::Never,
            EqeqeqNullPolicyJson::Ignore => EqeqeqNullPolicy::Ignore,
        }
    }
}

/// Operator-assignment mode accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OperatorAssignmentModeJson {
    /// Require shorthand assignment where possible.
    Always,
    /// Disallow shorthand assignment operators.
    Never,
}

impl From<OperatorAssignmentModeJson> for OperatorAssignmentMode {
    fn from(value: OperatorAssignmentModeJson) -> Self {
        match value {
            OperatorAssignmentModeJson::Always => OperatorAssignmentMode::Always,
            OperatorAssignmentModeJson::Never => OperatorAssignmentMode::Never,
        }
    }
}

/// Object shorthand mode accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ObjectShorthandModeJson {
    /// Require shorthand for methods and properties.
    Always,
    /// Require shorthand for methods only.
    Methods,
    /// Require shorthand for properties only.
    Properties,
    /// Disallow shorthand methods and properties.
    Never,
    /// Require object literals to use consistent shorthand style.
    Consistent,
    /// Require shorthand only when every eligible property can use it.
    ConsistentAsNeeded,
}

impl From<ObjectShorthandModeJson> for ObjectShorthandMode {
    fn from(value: ObjectShorthandModeJson) -> Self {
        match value {
            ObjectShorthandModeJson::Always => ObjectShorthandMode::Always,
            ObjectShorthandModeJson::Methods => ObjectShorthandMode::Methods,
            ObjectShorthandModeJson::Properties => ObjectShorthandMode::Properties,
            ObjectShorthandModeJson::Never => ObjectShorthandMode::Never,
            ObjectShorthandModeJson::Consistent => ObjectShorthandMode::Consistent,
            ObjectShorthandModeJson::ConsistentAsNeeded => ObjectShorthandMode::ConsistentAsNeeded,
        }
    }
}

/// Yoda mode accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum YodaModeJson {
    /// Require literal comparisons in Yoda form.
    Always,
    /// Disallow literal comparisons in Yoda form.
    Never,
}

impl From<YodaModeJson> for YodaMode {
    fn from(value: YodaModeJson) -> Self {
        match value {
            YodaModeJson::Always => YodaMode::Always,
            YodaModeJson::Never => YodaMode::Never,
        }
    }
}

/// JSON form of `grouped-accessor-pairs` ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum GroupedAccessorPairsOrderJson {
    /// Allow either adjacent accessor order.
    AnyOrder,
    /// Require getters before setters.
    GetBeforeSet,
    /// Require setters before getters.
    SetBeforeGet,
}

impl From<GroupedAccessorPairsOrderJson> for GroupedAccessorPairsOrder {
    fn from(value: GroupedAccessorPairsOrderJson) -> Self {
        match value {
            GroupedAccessorPairsOrderJson::AnyOrder => GroupedAccessorPairsOrder::AnyOrder,
            GroupedAccessorPairsOrderJson::GetBeforeSet => GroupedAccessorPairsOrder::GetBeforeSet,
            GroupedAccessorPairsOrderJson::SetBeforeGet => GroupedAccessorPairsOrder::SetBeforeGet,
        }
    }
}

/// JSON form of `sort-imports` member syntax groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SortImportsMemberSyntaxJson {
    /// Side-effect import syntax.
    None,
    /// Namespace import syntax.
    All,
    /// Multiple named imports.
    Multiple,
    /// Single default or named import.
    Single,
}

impl From<SortImportsMemberSyntaxJson> for SortImportsMemberSyntax {
    fn from(value: SortImportsMemberSyntaxJson) -> Self {
        match value {
            SortImportsMemberSyntaxJson::None => SortImportsMemberSyntax::None,
            SortImportsMemberSyntaxJson::All => SortImportsMemberSyntax::All,
            SortImportsMemberSyntaxJson::Multiple => SortImportsMemberSyntax::Multiple,
            SortImportsMemberSyntaxJson::Single => SortImportsMemberSyntax::Single,
        }
    }
}

/// JSON form of `prefer-const` destructuring policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PreferConstDestructuringJson {
    /// Report any const eligible binding in a destructuring.
    Any,
    /// Report destructuring bindings only when all are const eligible.
    All,
}

impl From<PreferConstDestructuringJson> for PreferConstDestructuring {
    fn from(value: PreferConstDestructuringJson) -> Self {
        match value {
            PreferConstDestructuringJson::Any => PreferConstDestructuring::Any,
            PreferConstDestructuringJson::All => PreferConstDestructuring::All,
        }
    }
}

/// Condition assignment policy for `no-cond-assign`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConditionAssignmentMode {
    /// Allow assignments only when wrapped in extra parentheses.
    #[default]
    ExceptParens,
    /// Disallow assignments anywhere inside the condition.
    Always,
}

/// Condition assignment policy accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ConditionAssignmentModeJson {
    /// Allow assignments only when wrapped in extra parentheses.
    ExceptParens,
    /// Disallow assignments anywhere inside the condition.
    Always,
}

impl From<ConditionAssignmentModeJson> for ConditionAssignmentMode {
    fn from(value: ConditionAssignmentModeJson) -> Self {
        match value {
            ConditionAssignmentModeJson::ExceptParens => ConditionAssignmentMode::ExceptParens,
            ConditionAssignmentModeJson::Always => ConditionAssignmentMode::Always,
        }
    }
}

/// Empty function kinds that `no-empty-function` may allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmptyFunctionKind {
    /// Regular function declarations and expressions.
    Functions,
    /// Arrow or lambda functions.
    ArrowFunctions,
    /// Generator functions.
    GeneratorFunctions,
    /// Ordinary methods.
    Methods,
    /// Generator methods.
    GeneratorMethods,
    /// Getter methods.
    Getters,
    /// Setter methods.
    Setters,
    /// Constructor methods.
    Constructors,
    /// Async functions.
    AsyncFunctions,
    /// Async methods.
    AsyncMethods,
    /// Override methods.
    OverrideMethods,
}

/// Empty function kinds accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum EmptyFunctionKindJson {
    /// Regular function declarations and expressions.
    Functions,
    /// Arrow or lambda functions.
    ArrowFunctions,
    /// Generator functions.
    GeneratorFunctions,
    /// Ordinary methods.
    Methods,
    /// Generator methods.
    GeneratorMethods,
    /// Getter methods.
    Getters,
    /// Setter methods.
    Setters,
    /// Constructor methods.
    Constructors,
    /// Async functions.
    AsyncFunctions,
    /// Async methods.
    AsyncMethods,
    /// Override methods.
    OverrideMethods,
}

impl From<EmptyFunctionKindJson> for EmptyFunctionKind {
    fn from(value: EmptyFunctionKindJson) -> Self {
        match value {
            EmptyFunctionKindJson::Functions => EmptyFunctionKind::Functions,
            EmptyFunctionKindJson::ArrowFunctions => EmptyFunctionKind::ArrowFunctions,
            EmptyFunctionKindJson::GeneratorFunctions => EmptyFunctionKind::GeneratorFunctions,
            EmptyFunctionKindJson::Methods => EmptyFunctionKind::Methods,
            EmptyFunctionKindJson::GeneratorMethods => EmptyFunctionKind::GeneratorMethods,
            EmptyFunctionKindJson::Getters => EmptyFunctionKind::Getters,
            EmptyFunctionKindJson::Setters => EmptyFunctionKind::Setters,
            EmptyFunctionKindJson::Constructors => EmptyFunctionKind::Constructors,
            EmptyFunctionKindJson::AsyncFunctions => EmptyFunctionKind::AsyncFunctions,
            EmptyFunctionKindJson::AsyncMethods => EmptyFunctionKind::AsyncMethods,
            EmptyFunctionKindJson::OverrideMethods => EmptyFunctionKind::OverrideMethods,
        }
    }
}

/// Warning comment matching locations accepted in linter JSON.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum WarningCommentLocationJson {
    /// Match terms only at the logical start of the comment.
    Start,
    /// Match terms anywhere in the comment body.
    Anywhere,
}

impl From<WarningCommentLocationJson> for WarningCommentLocation {
    fn from(value: WarningCommentLocationJson) -> Self {
        match value {
            WarningCommentLocationJson::Start => WarningCommentLocation::Start,
            WarningCommentLocationJson::Anywhere => WarningCommentLocation::Anywhere,
        }
    }
}

/// Bitwise operators accepted in linter JSON for the `no-bitwise` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum BitwiseOperatorJson {
    /// `&`
    #[serde(rename = "&")]
    And,
    /// `^`
    #[serde(rename = "^")]
    Xor,
    /// `|`
    #[serde(rename = "|")]
    Or,
    /// `~`
    #[serde(rename = "~")]
    Not,
    /// `<<`
    #[serde(rename = "<<")]
    ShiftLeft,
    /// `<<|`
    #[serde(rename = "<<|")]
    SaturatingShiftLeft,
    /// `>>`
    #[serde(rename = ">>")]
    ShiftRight,
    /// `>>>`
    #[serde(rename = ">>>")]
    UnsignedShiftRight,
    /// `&=`
    #[serde(rename = "&=")]
    AndAssign,
    /// `^=`
    #[serde(rename = "^=")]
    XorAssign,
    /// `|=`
    #[serde(rename = "|=")]
    OrAssign,
    /// `<<=`
    #[serde(rename = "<<=")]
    ShiftLeftAssign,
    /// `<<|=`
    #[serde(rename = "<<|=")]
    SaturatingShiftLeftAssign,
    /// `>>=`
    #[serde(rename = ">>=")]
    ShiftRightAssign,
    /// `>>>=`
    #[serde(rename = ">>>=")]
    UnsignedShiftRightAssign,
}

impl From<BitwiseOperatorJson> for BitwiseOperator {
    fn from(value: BitwiseOperatorJson) -> Self {
        match value {
            BitwiseOperatorJson::And => BitwiseOperator::And,
            BitwiseOperatorJson::Xor => BitwiseOperator::Xor,
            BitwiseOperatorJson::Or => BitwiseOperator::Or,
            BitwiseOperatorJson::Not => BitwiseOperator::Not,
            BitwiseOperatorJson::ShiftLeft => BitwiseOperator::ShiftLeft,
            BitwiseOperatorJson::SaturatingShiftLeft => BitwiseOperator::SaturatingShiftLeft,
            BitwiseOperatorJson::ShiftRight => BitwiseOperator::ShiftRight,
            BitwiseOperatorJson::UnsignedShiftRight => BitwiseOperator::UnsignedShiftRight,
            BitwiseOperatorJson::AndAssign => BitwiseOperator::AndAssign,
            BitwiseOperatorJson::XorAssign => BitwiseOperator::XorAssign,
            BitwiseOperatorJson::OrAssign => BitwiseOperator::OrAssign,
            BitwiseOperatorJson::ShiftLeftAssign => BitwiseOperator::ShiftLeftAssign,
            BitwiseOperatorJson::SaturatingShiftLeftAssign => {
                BitwiseOperator::SaturatingShiftLeftAssign
            }
            BitwiseOperatorJson::ShiftRightAssign => BitwiseOperator::ShiftRightAssign,
            BitwiseOperatorJson::UnsignedShiftRightAssign => {
                BitwiseOperator::UnsignedShiftRightAssign
            }
        }
    }
}
