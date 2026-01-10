/// Optimization level.
///
/// Controls which passes run and how aggressive they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    /// Debug: no optimizations.
    #[default]
    O0,
    /// Comptime/dev: fast local passes (constant fold, DCE, simplify CFG).
    O1,
    /// Release: full suite (inlining, escape analysis, devirtualization).
    O2,
    /// Hot paths: aggressive thresholds, loop unrolling.
    O3,
}

impl std::str::FromStr for OptimizationLevel {
    type Err = ();

    /// Parse from string (e.g., "O2" or "2").
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "O0" | "0" => Ok(Self::O0),
            "O1" | "1" => Ok(Self::O1),
            "O2" | "2" => Ok(Self::O2),
            "O3" | "3" => Ok(Self::O3),
            _ => Err(()),
        }
    }
}

/// Static metadata about an optimization pass.
#[derive(Debug, Clone, Copy)]
pub struct PassMetadata {
    /// Pass ID like "constant-fold".
    pub id: &'static str,
    /// Name like "ConstantFold".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

/// Base trait for all optimization passes.
///
/// Provides access to pass metadata.
pub trait Pass: Send + Sync {
    /// Get the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
}
