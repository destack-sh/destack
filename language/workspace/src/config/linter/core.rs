use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    LinterComplexityOptions, LinterCorrectnessOptions, LinterPerformanceOptions,
    LinterRestrictionOptions, LinterSecurityOptions, LinterStyleOptions, LinterSuspiciousOptions,
};

/// Lint rule categories.
///
/// Each category has a letter code used in lint identifiers (e.g., `LC002` for Correctness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ArrayTypeStyle {
    /// Prefer `T[]` syntax.
    #[default]
    Array,
    /// Prefer `Array<T>` syntax.
    Generic,
}

/// Preferred type definition syntax for the `consistent-type-definitions` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TypeDefinitionStyle {
    /// Prefer `type` aliases.
    #[default]
    Type,
    /// Prefer `interface` declarations.
    Interface,
}

/// Filename case style for the `filename-case` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum CyclomaticComplexityVariant {
    /// Count each non-default switch case as a branch.
    #[default]
    Classic,
    /// Count each switch as a single branch regardless of case count.
    Modified,
}

/// `this` parameter counting policy for `max-params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum MaxParamsCountThis {
    /// Never count the `this` parameter.
    Never,
    /// Count `this` unless it is explicitly typed as `void`.
    #[default]
    ExceptVoid,
    /// Always count the `this` parameter.
    Always,
}

/// Enforcement mode for `operator-assignment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum OperatorAssignmentMode {
    /// Require shorthand assignment where possible.
    #[default]
    Always,
    /// Disallow shorthand assignment operators.
    Never,
}

/// Enforcement mode for `object-shorthand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum YodaMode {
    /// Require literal comparisons in Yoda form.
    Always,
    /// Disallow literal comparisons in Yoda form.
    #[default]
    Never,
}

/// Ordering policy for `grouped-accessor-pairs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PreferConstDestructuring {
    /// Report any const eligible binding in a destructuring.
    #[default]
    Any,
    /// Report destructuring bindings only when all are const eligible.
    All,
}

/// Warning comment term matching location for `no-warning-comments`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum WarningCommentLocation {
    /// Match terms only at the logical start of the comment.
    #[default]
    Start,
    /// Match terms anywhere in the comment body.
    Anywhere,
}

/// Bitwise operators configurable for the `no-bitwise` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
            Self::ShiftRight => ">>",
            Self::UnsignedShiftRight => ">>>",
            Self::AndAssign => "&=",
            Self::XorAssign => "^=",
            Self::OrAssign => "|=",
            Self::ShiftLeftAssign => "<<=",
            Self::ShiftRightAssign => ">>=",
            Self::UnsignedShiftRightAssign => ">>>=",
        }
    }
}

/// Linter options.
///
/// Rule severity resolution order (highest precedence first):
/// 1. Individual rule overrides
/// 2. Category-level overrides
/// 3. Preset defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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
    /// Correctness-category options.
    pub correctness: LinterCorrectnessOptions,
    /// Suspicious-category options.
    pub suspicious: LinterSuspiciousOptions,
    /// Performance-category options.
    pub performance: LinterPerformanceOptions,
    /// Style-category options.
    pub style: LinterStyleOptions,
    /// Security-category options.
    pub security: LinterSecurityOptions,
    /// Complexity-category options.
    pub complexity: LinterComplexityOptions,
    /// Restriction-category options.
    pub restriction: LinterRestrictionOptions,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: LintPreset::Recommended,
            categories: IndexMap::new(),
            overrides: IndexMap::new(),
            include_declaration_files: false,
            correctness: LinterCorrectnessOptions::default(),
            suspicious: LinterSuspiciousOptions::default(),
            performance: LinterPerformanceOptions::default(),
            style: LinterStyleOptions::default(),
            security: LinterSecurityOptions::default(),
            complexity: LinterComplexityOptions::default(),
            restriction: LinterRestrictionOptions::default(),
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
        self.correctness.ignored_unused_parameter_prefixes = prefixes.into_iter().collect();
        self
    }

    /// Set allowed keyword prefixes for comment keyword comments.
    pub fn with_comment_keywords(mut self, keywords: impl IntoIterator<Item = String>) -> Self {
        self.style.comment_keywords = keywords.into_iter().collect();
        self
    }

    /// Set allowed tags for comment keyword comments.
    pub fn with_comment_keyword_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.style.comment_keyword_tags = tags.into_iter().collect();
        self
    }

    /// Set the minimum line count for separator heading comments.
    pub fn with_comment_separator_heading_min_lines(mut self, min_lines: usize) -> Self {
        self.style.comment_separator_heading_min_lines = min_lines;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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

/// Condition assignment policy for `no-cond-assign`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ConditionAssignmentMode {
    /// Allow assignments only when wrapped in extra parentheses.
    #[default]
    ExceptParens,
    /// Disallow assignments anywhere inside the condition.
    Always,
}

/// Empty function kinds that `no-empty-function` may allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
