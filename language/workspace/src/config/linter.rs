use indexmap::IndexMap;

/// Lint rule categories.
///
/// Each category has a letter code used in lint identifiers (e.g., `LC001` for Correctness).
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
    /// All rules enabled.
    All,
}

impl LintPreset {
    /// Parse a preset from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" | "off" => Some(Self::None),
            "recommended" => Some(Self::Recommended),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Recommended => "recommended",
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

    // complexity thresholds
    /// Maximum boolean parameters or fields.
    pub max_booleans: usize,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: usize,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: usize,
    /// Maximum nesting depth.
    pub max_depth: usize,
    /// Maximum lines per file.
    pub max_lines: usize,
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
    /// Maximum statements in a try block.
    pub max_try_block_statements: usize,

    // style options
    /// Preferred array type syntax.
    pub array_type: ArrayTypeStyle,
    /// Preferred type definition syntax.
    pub type_definition_style: TypeDefinitionStyle,
    /// Required catch clause error name.
    pub catch_error_name: String,
    /// Required filename case style.
    pub filename_case: FilenameCase,

    // restriction options
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Vec<f64>,
    /// Globals to restrict.
    pub restricted_globals: Vec<String>,
    /// Import paths to restrict.
    pub restricted_imports: Vec<String>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Vec<String>,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: LintPreset::Recommended,
            categories: IndexMap::new(),
            overrides: IndexMap::new(),
            // complexity
            max_booleans: 3,
            max_cognitive_complexity: 30,
            max_cyclomatic_complexity: 40,
            max_depth: 4,
            max_lines: 500,
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
            max_try_block_statements: 20,
            // style
            array_type: ArrayTypeStyle::default(),
            type_definition_style: TypeDefinitionStyle::default(),
            catch_error_name: "error".to_string(),
            filename_case: FilenameCase::default(),
            // restriction
            allowed_magic_numbers: vec![-1.0, 0.0, 1.0, 2.0],
            restricted_globals: Vec::new(),
            restricted_imports: Vec::new(),
            warning_comment_terms: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "HACK".to_string(),
            ],
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

    /// Get a rule's configured severity (returns None if not overridden).
    pub fn get_rule_severity(&self, rule: &str) -> Option<LintSeverity> {
        self.overrides.get(rule).copied()
    }

    /// Get a category's configured severity (returns None if not overridden).
    pub fn get_category_severity(&self, category: LintCategory) -> Option<LintSeverity> {
        self.categories.get(&category).copied()
    }

    /// Resolve effective severity for a rule given its category and default severity.
    ///
    /// Resolution order: rule override > category override > preset default
    pub fn resolve_severity(
        &self,
        rule_id: &str,
        category: LintCategory,
        default: LintSeverity,
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
                if category.is_recommended() {
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
