use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{AllocationSiteId, CallSiteId, ContinuationSiteId, CounterId, EdgeSiteId, Program};

const STANDARD_SAMPLE_BUCKET_LIMIT: u32 = 32;

/// Runtime profile aggregated by executable program sites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Profile {
    /// Runtime profile recording options.
    pub options: ProfileOptions,
    /// Explicit counter profiles indexed by executable counter id.
    pub counters: Vec<CounterProfile>,
    /// Heap allocation site profiles.
    pub allocations: Vec<AllocationProfile>,
    /// Function call site profiles.
    pub calls: Vec<CallProfile>,
    /// Control-flow edge site profiles.
    pub edges: Vec<EdgeProfile>,
    /// Continuation site profiles.
    pub continuations: Vec<ContinuationProfile>,
    /// Explicit sample profiles indexed by executable counter id.
    pub samples: Vec<SampleProfile>,
}

/// Runtime profile recording options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProfileOptions {
    /// Maximum exact value buckets kept for each sample site.
    pub sample_bucket_limit: u32,
}

/// Runtime profile for one explicit counter site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CounterProfile {
    /// Number of observed increments.
    pub count: u64,
}

/// Runtime profile for one heap allocation site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationProfile {
    /// Number of observed allocations.
    pub count: u64,
    /// Total requested allocation bytes.
    pub bytes: u64,
}

/// Runtime profile for one function call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallProfile {
    /// Number of observed calls.
    pub count: u64,
}

/// Runtime profile for one control-flow edge site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EdgeProfile {
    /// Number of observed transfers.
    pub count: u64,
}

/// Runtime profile for one continuation site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ContinuationProfile {
    /// Number of observed continuation captures.
    pub captured: u64,
    /// Number of observed continuation resumes.
    pub resumed: u64,
}

/// Runtime profile for one explicit sample site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SampleProfile {
    /// Number of observed samples.
    pub count: u64,
    /// Tracked sample value buckets.
    pub buckets: Vec<SampleBucket>,
    /// Samples not tracked individually.
    pub overflow: u64,
}

/// One tracked sample value bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SampleBucket {
    /// Raw sampled value bits.
    pub bits: u64,
    /// Number of samples with these bits.
    pub count: u64,
}

impl Profile {
    /// Create one empty profile shaped for a program.
    pub fn new(program: &Program, options: ProfileOptions) -> Self {
        let sections = program.sections();
        let sites = program.sites();
        let counter_count = sites.profile_counter_count(sections);

        Self {
            options,
            counters: vec![CounterProfile::new(); counter_count],
            allocations: vec![AllocationProfile::new(); sites.allocation_count(sections)],
            calls: vec![CallProfile::new(); sites.call_count(sections)],
            edges: vec![EdgeProfile::new(); sites.edge_count(sections)],
            continuations: vec![ContinuationProfile::new(); sites.continuation_count(sections)],
            samples: vec![SampleProfile::new(); counter_count],
        }
    }

    /// Return whether this profile has no runtime observations.
    pub fn is_empty(&self) -> bool {
        let counters_empty = self.counters.iter().all(CounterProfile::is_empty);
        let allocations_empty = self.allocations.iter().all(AllocationProfile::is_empty);
        let calls_empty = self.calls.iter().all(CallProfile::is_empty);
        let edges_empty = self.edges.iter().all(EdgeProfile::is_empty);
        let continuations_empty = self.continuations.iter().all(ContinuationProfile::is_empty);
        let samples_empty = self.samples.iter().all(SampleProfile::is_empty);

        counters_empty
            && allocations_empty
            && calls_empty
            && edges_empty
            && continuations_empty
            && samples_empty
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

    /// Record one continuation capture.
    #[inline]
    pub fn record_continuation_capture(&mut self, site: ContinuationSiteId) {
        self.continuations[site.index()].captured += 1;
    }

    /// Record one continuation resume.
    #[inline]
    pub fn record_continuation_resume(&mut self, site: ContinuationSiteId) {
        self.continuations[site.index()].resumed += 1;
    }

    /// Record one sampled cell value.
    #[inline]
    pub fn record_sample(&mut self, counter: CounterId, bits: u64) {
        self.samples[counter.index()].record(bits, self.options.sample_bucket_limit);
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

impl ContinuationProfile {
    /// Create one empty continuation profile.
    pub const fn new() -> Self {
        Self {
            captured: 0,
            resumed: 0,
        }
    }

    /// Return whether no continuation lifecycle event was observed.
    pub const fn is_empty(&self) -> bool {
        self.captured == 0 && self.resumed == 0
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

    /// Record one raw sample value.
    pub fn record(&mut self, bits: u64, bucket_limit: u32) {
        self.count += 1;

        if let Some(bucket) = self.buckets.iter_mut().find(|bucket| bucket.bits == bits) {
            bucket.count += 1;

            return;
        }

        if self.buckets.len() < bucket_limit as usize {
            self.buckets.push(SampleBucket { bits, count: 1 });
        } else {
            self.overflow += 1;
        }
    }
}
