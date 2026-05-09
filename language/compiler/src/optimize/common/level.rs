/// Optimization level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    /// Debug: no optimizations.
    #[default]
    O0,
    /// Comptime/dev: fast local passes.
    O1,
    /// Release: full optimization suite.
    O2,
    /// Hot paths: aggressive thresholds.
    O3,
    /// Maximal optimization.
    O4,
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
