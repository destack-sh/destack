use serde::{Deserialize, Serialize};
use tspp_core::{FxIndexMap, StringId};
use tspp_mir::FloatType;
use tspp_serde::Reflect;

use super::{AllocationSiteId, CallSiteId, CounterId, EdgeSiteId, Program, SamplerId, WordLayout};

const STANDARD_SAMPLE_BUCKET_LIMIT: u32 = 32;

/// Runtime profile aggregated by program sites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Profile {
    /// Runtime profile recording options.
    pub options: ProfileOptions,
    /// Explicit counter profiles indexed by program counter id.
    pub counters: Vec<CounterProfile>,
    /// Heap allocation site profiles.
    pub allocations: Vec<AllocationProfile>,
    /// Function call site profiles.
    pub calls: Vec<CallProfile>,
    /// Control-flow edge site profiles.
    pub edges: Vec<EdgeProfile>,
    /// Explicit sample profiles indexed by program sampler id.
    pub samples: Vec<SampleProfile>,
}

/// Runtime profile recording options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProfileOptions {
    /// Maximum exact value buckets kept for each sample site.
    pub sample_bucket_limit: u32,
}

/// Runtime profile for one explicit counter site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CounterProfile {
    /// Number of observed increments.
    pub count: u64,
}

/// Runtime profile for one heap allocation site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationProfile {
    /// Number of observed allocations.
    pub count: u64,
    /// Total requested allocation bytes.
    pub bytes: u64,
}

/// Runtime profile for one function call site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallProfile {
    /// Number of observed calls.
    pub count: u64,
}

/// Runtime profile for one control-flow edge site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EdgeProfile {
    /// Number of observed transfers.
    pub count: u64,
}

/// Runtime profile for one explicit sample site.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SampleProfile {
    /// Number of observed samples.
    pub count: u64,
    /// Tracked sample value buckets.
    pub buckets: Vec<SampleBucket>,
    /// Samples not tracked individually.
    pub overflow: u64,
}

/// One tracked sample value bucket.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SampleBucket {
    /// Exact sampled value key.
    pub key: SampleKey,
    /// Number of samples with this key.
    pub count: u64,
}

/// One exact sampled word key.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SampleKey(u64);

/// One decoded sampled word value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum SampleValue {
    /// Void value.
    Void,
    /// Boolean value.
    Boolean(bool),
    /// Unicode scalar value.
    Character(char),
    /// Signed integer value.
    Int {
        /// The integer payload.
        value: i64,
        /// The integer width in bits.
        width: u8,
    },
    /// Unsigned integer value.
    Uint {
        /// The integer payload.
        value: u64,
        /// The integer width in bits.
        width: u8,
    },
    /// Floating-point value.
    Float {
        /// The floating-point payload bits.
        bits: u64,
        /// The floating-point format.
        format: FloatType,
    },
    /// World memory reference value.
    Reference(u64),
    /// Function pointer value.
    FunctionPointer(u64),
    /// World-relative raw pointer value.
    Pointer(u64),
}

impl Profile {
    /// Create one empty profile shaped for a program.
    pub fn new(program: &Program, options: ProfileOptions) -> Self {
        let sections = program.sections();
        let sites = program.sites();
        let counter_count = sites.counter_count(sections);
        let sampler_count = sites.sampler_count(sections);

        Self {
            options,
            counters: vec![CounterProfile::new(); counter_count],
            allocations: vec![AllocationProfile::new(); sites.allocation_count(sections)],
            calls: vec![CallProfile::new(); sites.call_count(sections)],
            edges: vec![EdgeProfile::new(); sites.edge_count(sections)],
            samples: vec![SampleProfile::new(); sampler_count],
        }
    }

    /// Return the total count of every named counter, in first-site order.
    pub fn named_counters(&self, program: &Program) -> Vec<(StringId, u64)> {
        let sections = program.sections();
        let mut totals: FxIndexMap<StringId, u64> = FxIndexMap::default();
        for site in program.sites().counters(sections) {
            let Some(name) = site.name.get() else {
                continue;
            };
            *totals.entry(name).or_default() += self.counters[site.counter.index()].count;
        }

        totals.into_iter().collect()
    }

    /// Return whether this profile has no runtime observations.
    pub fn is_empty(&self) -> bool {
        let counters_empty = self.counters.iter().all(CounterProfile::is_empty);
        let allocations_empty = self.allocations.iter().all(AllocationProfile::is_empty);
        let calls_empty = self.calls.iter().all(CallProfile::is_empty);
        let edges_empty = self.edges.iter().all(EdgeProfile::is_empty);
        let samples_empty = self.samples.iter().all(SampleProfile::is_empty);

        counters_empty && allocations_empty && calls_empty && edges_empty && samples_empty
    }

    /// Record one explicit counter increment.
    #[inline]
    pub fn increment_counter(&mut self, counter: CounterId) {
        self.counters[counter.index()].count += 1;
    }

    /// Record one heap allocation.
    #[inline]
    pub fn record_allocation(&mut self, site: AllocationSiteId, bytes: u64) {
        let profile = &mut self.allocations[site.index()];

        profile.count += 1;
        profile.bytes += bytes;
    }

    /// Record one function call.
    #[inline]
    pub fn record_call(&mut self, site: CallSiteId) {
        self.calls[site.index()].count += 1;
    }

    /// Record one control-flow edge transfer.
    #[inline]
    pub fn record_edge(&mut self, site: EdgeSiteId) {
        self.edges[site.index()].count += 1;
    }

    /// Record one sampled word key.
    #[inline]
    pub fn record_sample(&mut self, sampler: SamplerId, key: u64) {
        self.samples[sampler.index()].record(SampleKey::new(key), self.options.sample_bucket_limit);
    }
}

impl SampleKey {
    /// Create one sample key from word bits.
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the raw sample key.
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Decode this key using one Program word layout.
    pub fn decode(self, layout: WordLayout) -> Option<SampleValue> {
        let value = match layout {
            WordLayout::Void => SampleValue::Void,
            WordLayout::Boolean => SampleValue::Boolean(self.0 != 0),
            WordLayout::Character => SampleValue::Character(char::from_u32(self.0 as u32)?),
            WordLayout::Int { width } => SampleValue::Int {
                value: self.signed(width),
                width,
            },
            WordLayout::Uint { width } => SampleValue::Uint {
                value: self.unsigned(width),
                width,
            },
            WordLayout::Float32 => SampleValue::Float {
                bits: self.unsigned(32),
                format: FloatType::Float32,
            },
            WordLayout::Float64 => SampleValue::Float {
                bits: self.0,
                format: FloatType::Float64,
            },
            WordLayout::Reference => SampleValue::Reference(self.0),
            WordLayout::FunctionPointer => SampleValue::FunctionPointer(self.0),
            WordLayout::Pointer => SampleValue::Pointer(self.0),
        };

        Some(value)
    }

    /// Decode this key as a truncated signed integer.
    const fn signed(self, width: u8) -> i64 {
        if width >= u64::BITS as u8 {
            return self.0 as i64;
        }

        let mask = (1u64 << width) - 1;
        let value = self.0 & mask;
        let sign_bit = 1u64 << (width - 1);

        if value & sign_bit != 0 {
            (value | !mask) as i64
        } else {
            value as i64
        }
    }

    /// Decode this key as a truncated unsigned integer.
    const fn unsigned(self, width: u8) -> u64 {
        if width >= u64::BITS as u8 {
            return self.0;
        }

        self.0 & ((1u64 << width) - 1)
    }
}

impl ProfileOptions {
    /// Standard runtime profile options.
    pub const STANDARD: Self = Self {
        sample_bucket_limit: STANDARD_SAMPLE_BUCKET_LIMIT,
    };
}

impl CounterProfile {
    /// Create one empty counter profile.
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    /// Return whether no counter increment was observed.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl AllocationProfile {
    /// Create one empty allocation profile.
    pub const fn new() -> Self {
        Self { count: 0, bytes: 0 }
    }

    /// Return whether no allocation was observed.
    pub const fn is_empty(&self) -> bool {
        self.count == 0 && self.bytes == 0
    }
}

impl CallProfile {
    /// Create one empty call profile.
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    /// Return whether no call was observed.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl EdgeProfile {
    /// Create one empty edge profile.
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    /// Return whether no edge transfer was observed.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl SampleProfile {
    /// Create one empty sample profile.
    pub fn new() -> Self {
        Self {
            count: 0,
            buckets: Vec::new(),
            overflow: 0,
        }
    }

    /// Return whether no sample was observed.
    pub fn is_empty(&self) -> bool {
        self.count == 0 && self.buckets.is_empty() && self.overflow == 0
    }

    /// Record one raw sample key.
    pub fn record(&mut self, key: SampleKey, bucket_limit: u32) {
        self.count += 1;

        if let Some(bucket) = self.buckets.iter_mut().find(|bucket| bucket.key == key) {
            bucket.count += 1;

            return;
        }

        if self.buckets.len() < bucket_limit as usize {
            self.buckets.push(SampleBucket { key, count: 1 });
        } else {
            self.overflow += 1;
        }
    }
}
