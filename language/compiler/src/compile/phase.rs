/// Phase label for compiler diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompilePhase {
    /// Import, parse, and bind source into base DIR.
    Import = 1,
    /// Resolve symbol references and profile environments.
    Resolve = 2,
    /// Declare, interface, analyze, and validate DIR.
    Analyze = 3,
    /// Elaborate checked DIR into lowered DIR form.
    Elaborate = 4,
    /// Execute comptime and patch DIR.
    Execute = 5,
    /// Lower patched DIR into MIR.
    Lower = 6,
    /// Optimize MIR.
    Optimize = 7,
    /// Generate emitted artifacts from compiler products.
    Generate = 8,
    /// Link emitted artifacts into package artifacts.
    Link = 9,
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
            Self::Import => "import",
            Self::Resolve => "resolve",
            Self::Analyze => "analyze",
            Self::Elaborate => "elaborate",
            Self::Execute => "execute",
            Self::Lower => "lower",
            Self::Optimize => "optimize",
            Self::Generate => "generate",
            Self::Link => "link",
        }
    }

    /// Return the description for this phase.
    pub fn description(&self) -> &str {
        match self {
            Self::Import => "import, parse, and bind source into DIR",
            Self::Resolve => "resolve symbol references and build semantic environments",
            Self::Analyze => "declare, interface, analyze, and validate",
            Self::Elaborate => "desugar and reify DIR",
            Self::Execute => "execute comptime code and patch DIR",
            Self::Lower => "lower DIR into MIR",
            Self::Optimize => "optimize MIR",
            Self::Generate => "generate emitted artifacts",
            Self::Link => "link emitted artifacts",
        }
    }

    /// Return the one letter code for this phase.
    pub fn letter(&self) -> char {
        match self {
            Self::Import => 'I',
            Self::Resolve => 'R',
            Self::Analyze => 'A',
            Self::Elaborate => 'E',
            Self::Execute => 'X',
            Self::Lower => 'M',
            Self::Optimize => 'O',
            Self::Generate => 'G',
            Self::Link => 'K',
        }
    }

    /// All phases in build order.
    pub const ALL: [CompilePhase; 9] = [
        Self::Import,
        Self::Resolve,
        Self::Analyze,
        Self::Elaborate,
        Self::Execute,
        Self::Lower,
        Self::Optimize,
        Self::Generate,
        Self::Link,
    ];

    /// Iterate over all phases in build order.
    pub fn all() -> impl Iterator<Item = CompilePhase> {
        Self::ALL.into_iter()
    }
}
