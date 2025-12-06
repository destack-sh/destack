use indexmap::IndexMap;

/// The linter options.
#[derive(Debug, Clone)]
pub struct LinterOptions {
    /// Whether linting is enabled.
    pub enabled: bool,
    /// Rule configuration.
    pub rules: LinterRules,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: LinterRules::default(),
        }
    }
}

impl LinterOptions {
    /// Create new linter options with linting enabled.
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Create new linter options with linting disabled.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            rules: LinterRules::default(),
        }
    }

    /// Set whether linting is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the rules configuration.
    pub fn with_rules(mut self, rules: LinterRules) -> Self {
        self.rules = rules;
        self
    }
}

/// Linter rules configuration.
#[derive(Debug, Clone)]
pub struct LinterRules {
    /// Enable the recommended rule set.
    pub recommended: bool,
    /// Individual rule overrides (rule name -> severity).
    pub overrides: IndexMap<String, RuleSeverity>,
}

impl Default for LinterRules {
    fn default() -> Self {
        Self {
            recommended: true,
            overrides: IndexMap::new(),
        }
    }
}

impl LinterRules {
    /// Create rules with recommended enabled.
    pub fn recommended() -> Self {
        Self {
            recommended: true,
            overrides: IndexMap::new(),
        }
    }

    /// Create rules with recommended disabled.
    pub fn none() -> Self {
        Self {
            recommended: false,
            overrides: IndexMap::new(),
        }
    }

    /// Set whether recommended rules are enabled.
    pub fn with_recommended(mut self, recommended: bool) -> Self {
        self.recommended = recommended;
        self
    }

    /// Set a rule's severity.
    pub fn with_rule(mut self, rule: impl Into<String>, severity: RuleSeverity) -> Self {
        self.overrides.insert(rule.into(), severity);
        self
    }

    /// Get a rule's severity (returns None if not overridden).
    pub fn get_severity(&self, rule: &str) -> Option<RuleSeverity> {
        self.overrides.get(rule).copied()
    }
}

/// Rule severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RuleSeverity {
    /// Rule is disabled.
    Off,
    /// Rule produces warnings.
    #[default]
    Warn,
    /// Rule produces errors.
    Error,
}

impl RuleSeverity {
    /// Whether this severity is enabled (not off).
    pub fn is_enabled(&self) -> bool {
        !matches!(self, RuleSeverity::Off)
    }

    /// Whether this severity is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, RuleSeverity::Error)
    }

    /// Whether this severity is a warning.
    pub fn is_warn(&self) -> bool {
        matches!(self, RuleSeverity::Warn)
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleSeverity::Off => "off",
            RuleSeverity::Warn => "warn",
            RuleSeverity::Error => "error",
        }
    }
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
