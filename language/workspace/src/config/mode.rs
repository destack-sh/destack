use indexmap::IndexMap;

use super::Condition;

/// Built-in source graph mode declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mode {
    /// Stable mode name.
    pub name: &'static str,
    /// Human-readable mode description.
    pub description: &'static str,
}

impl Mode {
    /// Development source graph mode.
    pub const DEV: Self = Self {
        name: "dev",
        description: "Development mode.",
    };

    /// Debug source graph mode.
    pub const DEBUG: Self = Self {
        name: "debug",
        description: "Debug mode.",
    };

    /// Production source graph mode.
    pub const PROD: Self = Self {
        name: "prod",
        description: "Production mode.",
    };

    /// Test source graph mode.
    pub const TEST: Self = Self {
        name: "test",
        description: "Test mode.",
    };

    /// Benchmark source graph mode.
    pub const BENCH: Self = Self {
        name: "bench",
        description: "Benchmark mode.",
    };

    /// Fuzzing source graph mode.
    pub const FUZZ: Self = Self {
        name: "fuzz",
        description: "Fuzzing mode.",
    };

    /// Simulation source graph mode.
    pub const SIM: Self = Self {
        name: "sim",
        description: "Simulation mode.",
    };

    /// Lint source graph mode.
    pub const LINT: Self = Self {
        name: "lint",
        description: "Lint mode.",
    };

    /// Built-in source graph modes.
    pub const BUILTINS: &'static [Self] = &[
        Self::DEV,
        Self::DEBUG,
        Self::PROD,
        Self::TEST,
        Self::BENCH,
        Self::FUZZ,
        Self::SIM,
        Self::LINT,
    ];

    /// Return normalized options for this built-in mode.
    pub fn condition(self) -> Condition {
        Condition {
            description: Some(self.description.to_string()),
            ..Condition::default()
        }
    }
}

/// Return the built-in source graph modes.
pub fn builtin_modes() -> IndexMap<String, Condition> {
    Mode::BUILTINS
        .iter()
        .map(|mode| (mode.name.to_string(), mode.condition()))
        .collect()
}
