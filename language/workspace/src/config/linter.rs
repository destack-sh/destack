use indexmap::IndexMap;
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
    /// Check callback positions in `no-misused-promises`.
    pub check_misused_promises_in_callbacks: bool,
    /// Check conditionals in `no-misused-promises`.
    pub check_misused_promises_in_conditionals: bool,
    /// Ignore explicit `void` wrappers in `no-confusing-void-expression`.
    pub no_confusing_void_expression_ignore_void_operator: bool,
    /// Await policy for the `return-await` rule.
    pub return_await_mode: ReturnAwaitMode,
    /// Parameter name prefixes ignored by `no-unused-parameters`.
    pub ignored_unused_parameter_prefixes: Vec<String>,

    // complexity thresholds
    /// Maximum boolean parameters or fields.
    pub max_booleans: usize,
    /// Maximum branches in a single conditional (if/match).
    pub max_branching_factor: usize,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: usize,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: usize,
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
    /// Maximum callback nesting.
    pub max_nested_callbacks: usize,
    /// Maximum function parameters.
    pub max_params: usize,
    /// Maximum statements per function.
    pub max_statements: usize,
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
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Vec<f64>,
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
            check_misused_promises_in_callbacks: true,
            check_misused_promises_in_conditionals: true,
            no_confusing_void_expression_ignore_void_operator: false,
            return_await_mode: ReturnAwaitMode::default(),
            ignored_unused_parameter_prefixes: vec!["_".to_string()],
            // complexity
            max_booleans: 3,
            max_branching_factor: 10,
            max_cognitive_complexity: 30,
            max_cyclomatic_complexity: 40,
            max_depth: 4,
            max_static_params: 4,
            max_lines: 500,
            max_lines_skip_comments: false,
            max_lines_skip_blank_lines: false,
            max_lines_per_function: 50,
            max_nested_callbacks: 4,
            max_params: 4,
            max_statements: 50,
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
            consistent_type_imports_prefer_type_imports: true,
            consistent_type_imports_prefer_inline_type_imports: false,
            consistent_type_imports_disallow_type_annotations: true,
            ignore_conditional_tests_in_prefer_nullish_coalescing: true,
            ignore_mixed_logical_expressions_in_prefer_nullish_coalescing: false,
            ignore_ternary_tests_in_prefer_nullish_coalescing: false,
            // restriction
            allowed_magic_numbers: vec![-1.0, 0.0, 1.0, 2.0],
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
pub struct DsConfigLinterJson {
    /// Whether linting is enabled. Default: true.
    pub enabled: Option<bool>,
    /// Rule configuration.
    #[serde(default)]
    pub rules: DsConfigLinterRulesJson,

    // complexity thresholds
    /// Maximum boolean parameters or fields.
    pub max_booleans: Option<usize>,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: Option<usize>,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: Option<usize>,
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
    /// Maximum callback nesting.
    pub max_nested_callbacks: Option<usize>,
    /// Maximum function parameters.
    pub max_params: Option<usize>,
    /// Maximum statements per function.
    pub max_statements: Option<usize>,
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
    /// Allow named callbacks in `prefer-arrow-callback`.
    pub allow_named_functions_in_prefer_arrow_callback: Option<bool>,
    /// Allow unbound `this` in `prefer-arrow-callback`.
    pub allow_unbound_this_in_prefer_arrow_callback: Option<bool>,
    /// Ignore explicit `void` wrappers in `no-confusing-void-expression`.
    pub no_confusing_void_expression_ignore_void_operator: Option<bool>,
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
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Option<Vec<f64>>,
    /// Globals to restrict.
    pub restricted_globals: Option<Vec<String>>,
    /// Import paths to restrict.
    pub restricted_imports: Option<Vec<String>>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Option<Vec<String>>,
    /// Module boundary constraints for module boundary aware lint rules.
    #[serde(alias = "architecture")]
    pub module_boundaries: Option<DsConfigLinterModuleBoundariesJson>,
}

impl DsConfigLinterJson {
    /// Apply linter options to a LinterOptions struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }
        self.rules.apply(options);

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
        if let Some(max_nested_callbacks) = self.max_nested_callbacks {
            options.max_nested_callbacks = max_nested_callbacks;
        }
        if let Some(max_params) = self.max_params {
            options.max_params = max_params;
        }
        if let Some(max_statements) = self.max_statements {
            options.max_statements = max_statements;
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
        if let Some(no_confusing_void_expression_ignore_void_operator) =
            self.no_confusing_void_expression_ignore_void_operator
        {
            options.no_confusing_void_expression_ignore_void_operator =
                no_confusing_void_expression_ignore_void_operator;
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
        if let Some(ref allowed_magic_numbers) = self.allowed_magic_numbers {
            options.allowed_magic_numbers = allowed_magic_numbers.clone();
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

/// Module boundary options for linter configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigLinterModuleBoundariesJson {
    /// Policy for modules that do not match any configured component.
    pub unknown_component_policy: Option<DiagnosticPolicyJson>,
    /// Declared module components.
    pub components: Option<Vec<DsConfigLinterModuleComponentJson>>,
    /// Allowed component to component dependency rules.
    pub rules: Option<Vec<DsConfigLinterModuleDependencyRuleJson>>,
    /// Explicit dependency exceptions.
    pub exceptions: Option<Vec<DsConfigLinterModuleDependencyExceptionJson>>,
}

impl DsConfigLinterModuleBoundariesJson {
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
pub struct DsConfigLinterModuleComponentJson {
    /// The unique component name.
    pub name: String,
    /// Glob patterns used to map modules into this component.
    #[serde(default, rename = "match")]
    pub path_patterns: Vec<String>,
}

impl From<&DsConfigLinterModuleComponentJson> for LintModuleComponent {
    fn from(value: &DsConfigLinterModuleComponentJson) -> Self {
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
pub struct DsConfigLinterModuleDependencyRuleJson {
    /// The source component name.
    pub from: String,
    /// The list of allowed destination component names.
    #[serde(default)]
    pub allow: Vec<String>,
}

impl From<&DsConfigLinterModuleDependencyRuleJson> for LintModuleDependencyRule {
    fn from(value: &DsConfigLinterModuleDependencyRuleJson) -> Self {
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
pub struct DsConfigLinterModuleDependencyExceptionJson {
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

impl From<&DsConfigLinterModuleDependencyExceptionJson> for LintModuleDependencyException {
    fn from(value: &DsConfigLinterModuleDependencyExceptionJson) -> Self {
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
pub struct DsConfigLinterRulesJson {
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

impl DsConfigLinterRulesJson {
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
