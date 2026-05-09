use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;

use super::{
    LinterComplexityJson, LinterComplexityOptions, LinterCorrectnessJson, LinterCorrectnessOptions,
    LinterPerformanceJson, LinterPerformanceOptions, LinterRestrictionJson,
    LinterRestrictionOptions, LinterSecurityJson, LinterSecurityOptions, LinterStyleJson,
    LinterStyleOptions, LinterSuspiciousJson, LinterSuspiciousOptions,
};

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
    /// Correctness-category options.
    #[serde(default, flatten)]
    pub correctness: LinterCorrectnessJson,
    /// Suspicious-category options.
    #[serde(default, flatten)]
    pub suspicious: LinterSuspiciousJson,
    /// Performance-category options.
    #[serde(default, flatten)]
    pub performance: LinterPerformanceJson,
    /// Style-category options.
    #[serde(default, flatten)]
    pub style: LinterStyleJson,
    /// Security-category options.
    #[serde(default, flatten)]
    pub security: LinterSecurityJson,
    /// Complexity-category options.
    #[serde(default, flatten)]
    pub complexity: LinterComplexityJson,
    /// Restriction-category options.
    #[serde(default, flatten)]
    pub restriction: LinterRestrictionJson,
}

impl LinterJson {
    /// Validate configuration values that need semantic checking.
    pub fn validate(&self) -> Result<(), String> {
        self.correctness.validate()?;
        self.suspicious.validate()?;
        self.performance.validate()?;
        self.style.validate()?;
        self.security.validate()?;
        self.complexity.validate()?;
        self.restriction.validate()?;

        Ok(())
    }

    /// Apply linter options to a LinterOptions struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }

        self.rules.apply(options);
        self.correctness.apply(options);
        self.suspicious.apply(options);
        self.performance.apply(options);
        self.style.apply(options);
        self.security.apply(options);
        self.complexity.apply(options);
        self.restriction.apply(options);
    }
}

/// Validate one list of regex patterns.
pub(crate) fn validate_regex_patterns(
    field_name: &str,
    patterns: Option<&[String]>,
) -> Result<(), String> {
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
pub(crate) fn validate_single_character_strings(
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
pub(crate) fn validate_exact_enum_order<T: PartialEq + Copy>(
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
