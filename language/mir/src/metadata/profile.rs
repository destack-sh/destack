use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Edge, Symbol};

/// Loaded profile-guided optimization data for a program.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Per-function profile, by persistent symbol.
    pub functions: HashMap<Symbol, FunctionProfile>,
    /// Per-global profile, by persistent symbol.
    pub globals: HashMap<Symbol, GlobalProfile>,
}

impl Profile {
    /// Create an empty profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true when no per-symbol profile data is present.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty() && self.globals.is_empty()
    }

    /// Look up one function's profile by symbol.
    pub fn function(&self, symbol: Symbol) -> Option<&FunctionProfile> {
        self.functions.get(&symbol)
    }

    /// Look up one global's profile by symbol.
    pub fn global(&self, symbol: Symbol) -> Option<&GlobalProfile> {
        self.globals.get(&symbol)
    }
}

/// Profile data for one function, addressed by its persistent symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionProfile {
    /// Control-flow hash guarding against stale application.
    pub hash: FunctionHash,
    /// Function entry execution count.
    pub entry: Count,
    /// Observed control-flow edge counts.
    pub edges: HashMap<Edge, Count>,
    /// Per-counter execution counts, indexed by [`CounterId`].
    pub counts: Vec<Count>,
    /// Observed value-profiling sites, by counter.
    pub values: HashMap<CounterId, ValueProfile>,
}

/// Profile data for one global, addressed by its persistent symbol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalProfile {
    /// Writes observed after initialization; zero means effectively constant.
    pub writes: Count,
    /// Read accesses observed, for layout and colocation.
    pub reads: Count,
}

/// Observed runtime values recorded at one value-profiling site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueProfile {
    /// Indirect and virtual call target distribution.
    Calls(Histogram<Symbol>),
    /// Observed runtime type distribution at a dynamic site.
    Types(Histogram<Symbol>),
    /// Scalar value or size distribution.
    Scalars(Histogram<i64>),
    /// Allocation size and survival behavior.
    Alloc(Allocation),
    /// Suspension behavior at a suspension point.
    Suspend(Suspension),
}

/// Observed frequency distribution over a domain at one site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Histogram<T> {
    /// Observed entries and how often each occurred.
    pub buckets: Vec<(T, Count)>,
    /// Count attributed to entries not individually tracked.
    pub unknown: Count,
}

/// Observed allocation behavior at one allocation site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Allocation {
    /// Observed payload sizes; its total is the number of allocations seen.
    pub size: Histogram<i64>,
    /// How many of those allocations were promoted past the young generation.
    pub survived: Count,
}

/// Observed suspension behavior at one suspension point.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suspension {
    /// Executions of the suspension point.
    pub reached: Count,
    /// Of those, how many actually parked (handed back a continuation).
    pub parked: Count,
    /// Of the parked frames, how many were resumed; the rest were dropped.
    pub resumed: Count,
}

/// Execution count from profile data.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Count(
    /// The raw count value.
    pub u64,
);

impl Count {
    /// Create one execution count.
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw count value.
    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identifier for one emitted profile counter, positional within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CounterId(
    /// The zero-based profile counter index.
    pub u32,
);

/// Structural hash of a function's profiled control flow, for stale detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionHash(
    /// The structural hash value.
    pub u64,
);
