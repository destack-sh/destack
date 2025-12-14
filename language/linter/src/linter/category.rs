/// Lint rule categories.
///
/// Each category has a letter code used in lint identifiers (e.g., `LC001` for Correctness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintCategory {
    /// Correctness lints detect likely bugs and logic errors.
    /// These are high-confidence issues that are almost always wrong.
    /// Letter: C
    Correctness,

    /// Suspicious lints detect code that is likely unintentional.
    /// These patterns are usually bugs but may occasionally be intentional.
    /// Letter: U (sUspicious)
    Suspicious,

    /// Performance lints detect inefficient patterns.
    /// The code is correct but could be faster or use less memory.
    /// Letter: P
    Performance,

    /// Style lints enforce consistent coding style.
    /// These are subjective preferences, not correctness issues.
    /// Letter: Y (stYle)
    Style,

    /// Security lints detect potential vulnerabilities.
    /// These patterns may expose the application to attacks.
    /// Letter: S
    Security,

    /// Complexity lints detect overly complex code.
    /// High complexity makes code harder to understand and maintain.
    /// Letter: X (compleXity)
    Complexity,

    /// Restriction lints enforce project-specific restrictions.
    /// These are opt-in rules that ban certain patterns by choice.
    /// Letter: R
    Restriction,

    /// Pedantic lints are very strict or opinionated.
    /// These may have false positives or be too noisy for some projects.
    /// Letter: D (peDantic)
    Pedantic,
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
            Self::Pedantic => 'D',
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
            Self::Pedantic => "pedantic",
        }
    }

    /// Get the description of the category.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Correctness => "detects likely bugs and logic errors",
            Self::Suspicious => "detects code that is likely unintentional",
            Self::Performance => "detects inefficient patterns",
            Self::Style => "enforces consistent coding style",
            Self::Security => "detects potential vulnerabilities",
            Self::Complexity => "detects overly complex code",
            Self::Restriction => "enforces project-specific restrictions",
            Self::Pedantic => "very strict or opinionated checks",
        }
    }

    /// Whether this category is part of the "recommended" set.
    pub const fn is_recommended(&self) -> bool {
        matches!(
            self,
            Self::Correctness | Self::Suspicious | Self::Security
        )
    }

    /// Default severity for rules in this category.
    pub const fn default_severity(&self) -> destack_source::RuleSeverity {
        use destack_source::RuleSeverity;
        match self {
            Self::Correctness => RuleSeverity::Error,
            Self::Suspicious => RuleSeverity::Warn,
            Self::Performance => RuleSeverity::Warn,
            Self::Style => RuleSeverity::Warn,
            Self::Security => RuleSeverity::Error,
            Self::Complexity => RuleSeverity::Warn,
            Self::Restriction => RuleSeverity::Off, // opt-in
            Self::Pedantic => RuleSeverity::Off,    // opt-in
        }
    }

    /// Parse a category from its name.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "correctness" => Some(Self::Correctness),
            "suspicious" => Some(Self::Suspicious),
            "performance" => Some(Self::Performance),
            "style" => Some(Self::Style),
            "security" => Some(Self::Security),
            "complexity" => Some(Self::Complexity),
            "restriction" => Some(Self::Restriction),
            "pedantic" => Some(Self::Pedantic),
            _ => None,
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
        Self::Pedantic,
    ];
}

impl std::fmt::Display for LintCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
