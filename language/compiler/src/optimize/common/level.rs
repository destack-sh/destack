/// Optimization level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    /// Debug build level with canonicalization only.
    #[default]
    O0,
    /// Fast local optimization level.
    O1,
    /// Standard release optimization level.
    O2,
    /// Aggressive release optimization level.
    O3,
    /// Maximum Destack optimization level.
    O4,
}

impl OptimizationLevel {
    /// Return the optimization program scope.
    pub const fn program_scope(self) -> ProgramScope {
        match self {
            Self::O0 | Self::O1 => ProgramScope::Module,
            Self::O2 | Self::O3 | Self::O4 => ProgramScope::Program,
        }
    }

    /// Return true when this level uses program analysis.
    pub const fn uses_program_analysis(self) -> bool {
        matches!(self.program_scope(), ProgramScope::Program)
    }

    /// Return the loop unroll threshold in instructions.
    pub const fn unroll_threshold(self) -> usize {
        match self {
            Self::O0 | Self::O1 => 0,
            Self::O2 => 200,
            Self::O3 => 300,
            Self::O4 => 400,
        }
    }

    /// Return the inline budget scale percent.
    pub const fn inline_budget_scale_percent(self) -> u64 {
        match self {
            Self::O0 => 50,
            Self::O1 => 75,
            Self::O2 => 100,
            Self::O3 => 140,
            Self::O4 => 180,
        }
    }
}

impl From<destack_repository::OptimizeLevel> for OptimizationLevel {
    /// Convert a repository optimization level to compiler policy.
    fn from(level: destack_repository::OptimizeLevel) -> Self {
        match level {
            destack_repository::OptimizeLevel::O0 => Self::O0,
            destack_repository::OptimizeLevel::O1 => Self::O1,
            destack_repository::OptimizeLevel::O2 => Self::O2,
            destack_repository::OptimizeLevel::O3 => Self::O3,
            destack_repository::OptimizeLevel::O4 => Self::O4,
        }
    }
}

impl std::str::FromStr for OptimizationLevel {
    type Err = ();

    /// Parse an optimization level.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "O0" | "0" => Ok(Self::O0),
            "O1" | "1" => Ok(Self::O1),
            "O2" | "2" => Ok(Self::O2),
            "O3" | "3" => Ok(Self::O3),
            "O4" | "4" => Ok(Self::O4),
            _ => Err(()),
        }
    }
}

/// Optimization program scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramScope {
    /// One module is the optimization program.
    Module,
    /// The target-built program is the optimization program.
    Program,
}
