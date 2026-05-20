/// Phase label for compiler diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompilePhase {
    /// Bind source into base DIR.
    Bind = 1,
    /// Resolve imports into dependency tables.
    Import = 2,
    /// Expand macros into DIR patches.
    Expand = 3,
    /// Resolve exports over expanded DIR.
    Export = 4,
    /// Resolve imports into symbol targets.
    Resolve = 5,
    /// Check expanded DIR.
    Check = 6,
    /// Materialize comptime and patch DIR.
    Materialize = 7,
    /// Elaborate materialized DIR into lowered DIR form.
    Elaborate = 8,
    /// Lower patched DIR into MIR.
    Lower = 9,
    /// Verify lowered MIR.
    Verify = 10,
    /// Optimize MIR.
    Optimize = 11,
    /// Generate emitted artifacts from compiler products.
    Generate = 12,
    /// Link emitted artifacts into package artifacts.
    Link = 13,
}

impl std::fmt::Display for CompilePhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.letter())
    }
}

impl CompilePhase {
    /// Return the numeric code for this phase.
    pub fn code(&self) -> u8 {
        *self as u8
    }

    /// Return the display name for this phase.
    pub fn name(&self) -> &str {
        match self {
            Self::Bind => "bind",
            Self::Import => "import",
            Self::Expand => "expand",
            Self::Export => "export",
            Self::Resolve => "resolve",
            Self::Check => "check",
            Self::Elaborate => "elaborate",
            Self::Materialize => "materialize",
            Self::Lower => "lower",
            Self::Verify => "verify",
            Self::Optimize => "optimize",
            Self::Generate => "generate",
            Self::Link => "link",
        }
    }

    /// Return the description for this phase.
    pub fn description(&self) -> &str {
        match self {
            Self::Bind => "parse and bind source into DIR",
            Self::Import => "resolve imports into dependency tables",
            Self::Expand => "expand macros into DIR patches",
            Self::Export => "resolve exports over expanded DIR",
            Self::Resolve => "resolve imports into symbol targets",
            Self::Check => "check expanded DIR",
            Self::Materialize => "materialize comptime code and patch DIR",
            Self::Elaborate => "desugar and reify DIR",
            Self::Lower => "lower DIR into MIR",
            Self::Verify => "verify MIR semantic invariants",
            Self::Optimize => "optimize MIR",
            Self::Generate => "generate emitted artifacts",
            Self::Link => "link emitted artifacts",
        }
    }

    /// Return the one letter code for this phase.
    pub fn letter(&self) -> char {
        match self {
            Self::Bind => 'B',
            Self::Import => 'I',
            Self::Expand => 'X',
            Self::Export => 'T',
            Self::Resolve => 'R',
            Self::Check => 'C',
            Self::Elaborate => 'E',
            Self::Materialize => 'M',
            Self::Lower => 'L',
            Self::Verify => 'V',
            Self::Optimize => 'O',
            Self::Generate => 'G',
            Self::Link => 'K',
        }
    }

    /// All phases in build order.
    pub const ALL: [CompilePhase; 13] = [
        Self::Bind,
        Self::Import,
        Self::Expand,
        Self::Export,
        Self::Resolve,
        Self::Check,
        Self::Materialize,
        Self::Elaborate,
        Self::Lower,
        Self::Verify,
        Self::Optimize,
        Self::Generate,
        Self::Link,
    ];

    /// Iterate over all phases in build order.
    pub fn all() -> impl Iterator<Item = CompilePhase> {
        Self::ALL.into_iter()
    }
}
