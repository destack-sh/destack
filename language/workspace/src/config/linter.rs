use indexmap::IndexMap;

/// Lint rule preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LintPreset {
    /// No rules enabled by default.
    None,
    /// Recommended rules enabled (default).
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
    pub categories: IndexMap<String, LintSeverity>,
    /// Individual rule severity overrides.
    pub overrides: IndexMap<String, LintSeverity>,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: LintPreset::Recommended,
            categories: IndexMap::new(),
            overrides: IndexMap::new(),
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
            enabled: true,
            preset: LintPreset::None,
            categories: IndexMap::new(),
            overrides: IndexMap::new(),
        }
    }

    /// Create options with all rules enabled.
    pub fn all() -> Self {
        Self {
            enabled: true,
            preset: LintPreset::All,
            categories: IndexMap::new(),
            overrides: IndexMap::new(),
        }
    }

    /// Set the preset.
    pub fn with_preset(mut self, preset: LintPreset) -> Self {
        self.preset = preset;
        self
    }

    /// Set a category's severity.
    pub fn with_category(mut self, category: impl Into<String>, severity: LintSeverity) -> Self {
        self.categories.insert(category.into(), severity);
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
    pub fn get_category_severity(&self, category: &str) -> Option<LintSeverity> {
        self.categories.get(category).copied()
    }

    /// Resolve effective severity for a rule given its category and default severity.
    ///
    /// Resolution order: rule override > category override > preset default
    pub fn resolve_severity(
        &self,
        rule_id: &str,
        category: &str,
        default: LintSeverity,
        is_recommended: bool,
    ) -> LintSeverity {
        // rule override takes precedence
        if let Some(severity) = self.overrides.get(rule_id) {
            return *severity;
        }

        // category override
        if let Some(severity) = self.categories.get(category) {
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
