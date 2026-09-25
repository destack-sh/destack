use tspp_core::{FxIndexMap, StringId};

use serde::{Deserialize, Serialize};

use tspp_serde::Reflect;

use crate::{Edge, FunctionId, Instruction, LocalNodeId, Point, Symbol, TypeId, Value};

/// Loaded profile-guided optimization data for a program.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Profile {
    /// Per-function profile, by persistent symbol.
    pub functions: FxIndexMap<Symbol, FunctionProfile>,
    /// Per-global profile, by persistent symbol.
    pub globals: FxIndexMap<Symbol, GlobalProfile>,
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

/// Static profile site table for one MIR module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProfileTable {
    /// Per-function profile site tables.
    pub functions: FxIndexMap<FunctionId, FunctionProfileTable>,
}

impl ProfileTable {
    /// Create an empty profile table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return one function's profile site table.
    pub fn function(&self, function: FunctionId) -> Option<&FunctionProfileTable> {
        self.functions.get(&function)
    }

    /// Insert one function's profile site table.
    pub fn insert_function(
        &mut self,
        function: FunctionId,
        profile: FunctionProfileTable,
    ) -> Option<FunctionProfileTable> {
        self.functions.insert(function, profile)
    }

    /// Return one counter site's counter id.
    pub fn counter(&self, function: FunctionId, site: &CounterSite) -> Option<CounterId> {
        self.function(function)
            .and_then(|profile| profile.counter(site))
    }

    /// Return one sample site's sampler id.
    pub fn sampler(&self, function: FunctionId, site: &SampleSite) -> Option<SamplerId> {
        self.function(function)
            .and_then(|profile| profile.sampler(site))
    }
}

/// Static profile site table for one function.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionProfileTable {
    /// Control-flow hash guarding against stale profile application.
    pub hash: FunctionHash,
    /// Counter sites indexed by counter id.
    pub counters: Vec<CounterSite>,
    /// Sample sites indexed by sampler id.
    pub samplers: Vec<SampleSite>,
}

impl FunctionProfileTable {
    /// Create an empty function profile table.
    pub fn new(hash: FunctionHash) -> Self {
        Self {
            hash,
            counters: Vec::new(),
            samplers: Vec::new(),
        }
    }

    /// Insert one counter site and return its counter id.
    pub fn insert_counter(&mut self, site: CounterSite) -> CounterId {
        let counter = CounterId(self.counters.len() as u32);
        self.counters.push(site);

        counter
    }

    /// Return one counter site by counter id.
    pub fn counter_site(&self, counter: CounterId) -> Option<&CounterSite> {
        self.counters.get(counter.index())
    }

    /// Return one counter site's counter id.
    pub fn counter(&self, site: &CounterSite) -> Option<CounterId> {
        self.counters
            .iter()
            .position(|candidate| candidate == site)
            .map(|index| CounterId(index as u32))
    }

    /// Insert one sample site and return its sampler id.
    pub fn insert_sampler(&mut self, site: SampleSite) -> SamplerId {
        let sampler = SamplerId(self.samplers.len() as u32);
        self.samplers.push(site);

        sampler
    }

    /// Return one sample site by sampler id.
    pub fn sample_site(&self, sampler: SamplerId) -> Option<&SampleSite> {
        self.samplers.get(sampler.index())
    }

    /// Return one sample site's sampler id.
    pub fn sampler(&self, site: &SampleSite) -> Option<SamplerId> {
        self.samplers
            .iter()
            .position(|candidate| candidate == site)
            .map(|index| SamplerId(index as u32))
    }
}

/// Semantic meaning of one profile counter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CounterSite {
    /// Function entry execution count.
    Entry,
    /// Control-flow edge count.
    Edge(Edge),
    /// User instrument counted under its declared name.
    Named(StringId),
}

/// Semantic meaning of one profile sampler.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum SampleSite {
    /// Value distribution for one SSA value.
    Value(Value),
    /// Observed call target distribution for one callsite.
    CallTarget(Point),
    /// Observed receiver type distribution for one callsite.
    ReceiverType(Point),
    /// Observed allocation behavior for one allocation instruction.
    Allocation(LocalNodeId<Instruction>),
    /// User instrument sampled under its declared name.
    Named(StringId),
}

/// Profile data for one function, addressed by its persistent symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionProfile {
    /// Control-flow hash guarding against stale application.
    pub hash: FunctionHash,
    /// Function entry execution count.
    pub entry: Count,
    /// Observed control-flow edge counts.
    pub edges: FxIndexMap<Edge, Count>,
    /// Per-counter execution counts, indexed by [`CounterId`].
    pub counts: Vec<Count>,
    /// Observed value profiles indexed by sampler id.
    pub values: FxIndexMap<SamplerId, ValueProfile>,
}

impl FunctionProfile {
    /// Return one edge count, or zero when the edge has no profile.
    pub fn edge(&self, edge: Edge) -> u64 {
        self.edges.get(&edge).map_or(0, |count| count.get())
    }
}

/// Profile data for one global, addressed by its persistent symbol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GlobalProfile {
    /// Writes observed after initialization, with zero indicating no observed writes.
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
    Types(Histogram<TypeId>),
    /// Scalar value or size distribution.
    Scalars(Histogram<i64>),
    /// Allocation size and survival behavior.
    Allocation(Allocation),
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
    /// How many of those allocations survived a collection.
    pub survived: Count,
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

impl CounterId {
    /// Return this id as a dense function-local index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for one emitted profile sampler, positional within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SamplerId(
    /// The zero-based profile sampler index.
    pub u32,
);

impl SamplerId {
    /// Return this id as a dense function-local index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Structural hash of a function's profiled control flow, for stale detection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionHash(
    /// The structural hash value.
    pub u64,
);
