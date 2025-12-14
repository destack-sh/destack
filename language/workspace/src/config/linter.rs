use indexmap::IndexMap;

/// The linter options.
#[derive(Debug, Clone)]
pub struct LinterOptions {
    /// Whether linting is enabled.
    pub enabled: bool,
    /// Rule configuration.
    pub rules: LinterRules, // nocheckin: this feels awkward, inline LinterRules struct?
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: LinterRules::default(),
        }
    }
}

/// Linter rules configuration.
#[derive(Debug, Clone)]
pub struct LinterRules {
    /// Enable the recommended rule set. // nocheckin: better way of doing categories?
    pub recommended: bool,
    /// Individual rule overrides (rule name -> severity).
    pub overrides: IndexMap<String, LintSeverity>,
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
    pub fn with_rule(mut self, rule: impl Into<String>, severity: LintSeverity) -> Self {
        self.overrides.insert(rule.into(), severity);
        self
    }

    /// Get a rule's severity (returns None if not overridden).
    pub fn get_severity(&self, rule: &str) -> Option<LintSeverity> {
        self.overrides.get(rule).copied()
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
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
