use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{CallSite, Edge, FunctionId, Instruction, LocalNodeId, Symbol, Type, Value};

/// Loaded profile-guided optimization data for a program.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
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

/// Static profile counter table for one MIR module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProfileTable {
    /// Per-function profile counter tables.
    pub functions: HashMap<FunctionId, FunctionProfileTable>,
}

impl ProfileTable {
    /// Create an empty profile table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return one function's profile counter table.
    pub fn function(&self, function: FunctionId) -> Option<&FunctionProfileTable> {
        self.functions.get(&function)
    }

    /// Insert one function's profile counter table.
    pub fn insert_function(
        &mut self,
        function: FunctionId,
        profile: FunctionProfileTable,
    ) -> Option<FunctionProfileTable> {
        self.functions.insert(function, profile)
    }

    /// Return one profile point's counter id.
    pub fn counter(&self, function: FunctionId, point: &ProfilePoint) -> Option<CounterId> {
        self.function(function)
            .and_then(|profile| profile.counter(point))
    }
}

/// Static profile counter table for one function.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionProfileTable {
    /// Control-flow hash guarding against stale profile application.
    pub hash: FunctionHash,
    /// Profile points indexed by counter id.
    pub points: Vec<ProfilePoint>,
}

impl FunctionProfileTable {
    /// Create an empty function profile table.
    pub fn new(hash: FunctionHash) -> Self {
        Self {
            hash,
            points: Vec::new(),
        }
    }

    /// Insert one profile point and return its counter id.
    pub fn insert(&mut self, point: ProfilePoint) -> CounterId {
        let counter = CounterId(self.points.len() as u32);
        self.points.push(point);

        counter
    }

    /// Return one profile point by counter id.
    pub fn point(&self, counter: CounterId) -> Option<&ProfilePoint> {
        self.points.get(counter.0 as usize)
    }

    /// Return one profile point's counter id.
    pub fn counter(&self, point: &ProfilePoint) -> Option<CounterId> {
        self.points
            .iter()
            .position(|candidate| candidate == point)
            .map(|index| CounterId(index as u32))
    }
}

/// Semantic meaning of one profile counter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ProfilePoint {
    /// Function entry execution count.
    Entry,
    /// Control-flow edge count.
    Edge(Edge),
    /// Value distribution for one SSA value.
    Value(Value),
    /// Observed call target distribution for one callsite.
    CallTarget(CallSite),
    /// Observed receiver type distribution for one callsite.
    ReceiverType(CallSite),
    /// Observed allocation behavior for one allocation instruction.
    Allocation(LocalNodeId<Instruction>),
    /// Suspension behavior for one instruction.
    Suspension(LocalNodeId<Instruction>),
}

/// Profile data for one function, addressed by its persistent symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GlobalProfile {
    /// Writes observed after initialization; zero means effectively constant.
    pub writes: Count,
    /// Read accesses observed, for layout and colocation.
    pub reads: Count,
}

/// Observed runtime values recorded at one value-profiling site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ValueProfile {
    /// Indirect and virtual call target distribution.
    Calls(Histogram<Symbol>),
    /// Observed exact runtime type distribution at a dynamic site.
    Types(Histogram<LocalNodeId<Type>>),
    /// Scalar value or size distribution.
    Scalars(Histogram<i64>),
    /// Allocation size and survival behavior.
    Allocation(Allocation),
    /// Suspension behavior at a suspension point.
    Suspend(Suspension),
}

/// Observed frequency distribution over a domain at one site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Histogram<T> {
    /// Observed entries and how often each occurred.
    pub buckets: Vec<(T, Count)>,
    /// Count attributed to entries not individually tracked.
    pub unknown: Count,
}

impl<T> Histogram<T> {
    /// Return the total observed count.
    pub fn total(&self) -> Count {
        let buckets = self
            .buckets
            .iter()
            .map(|(_, count)| count.get())
            .sum::<u64>();

        Count::new(buckets + self.unknown.get())
    }

    /// Return the most observed bucket when one exists.
    pub fn dominant(&self) -> Option<(&T, Count)> {
        self.buckets
            .iter()
            .max_by_key(|(_, count)| count.get())
            .map(|(value, count)| (value, *count))
    }
}

/// Observed allocation behavior at one allocation instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Allocation {
    /// Observed payload sizes; its total is the number of allocations seen.
    pub size: Histogram<i64>,
    /// How many of those allocations were promoted past the young generation.
    pub survived: Count,
}

/// Observed suspension behavior at one suspension point.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CounterId(
    /// The zero-based profile counter index.
    pub u32,
);

/// Structural hash of a function's profiled control flow, for stale detection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionHash(
    /// The structural hash value.
    pub u64,
);
