use serde::{Deserialize, Serialize};

/// Activation behavior for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivationWindow {
    /// Activate the rule immediately.
    Immediate,
    /// Activate once virtual time reaches this timestamp.
    AtVirtualNs {
        /// Activation timestamp in virtual nanoseconds.
        virtual_ns: u64,
    },
    /// Activate once this number of matching calls has elapsed.
    AfterCallCount {
        /// Matching call count before activation.
        call_count: u64,
    },
}

/// Lifetime behavior for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lifetime {
    /// Keep the rule active until explicitly disabled.
    UntilDisabled,
    /// Keep the rule active for this virtual duration.
    ForDurationNs {
        /// Active duration in virtual nanoseconds.
        duration_ns: u64,
    },
    /// Keep the rule active for this number of matching calls.
    ForCallCount {
        /// Active call budget before expiration.
        call_count: u64,
    },
}

/// Hook for runtime effect rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hook {
    /// Trigger before invoking one binding implementation.
    BindingBefore,
    /// Trigger after invoking one binding implementation.
    BindingAfter,
    /// Trigger when one task is enqueued.
    SchedulerEnqueue,
    /// Trigger when one task is dequeued.
    SchedulerDequeue,
    /// Trigger when one timer fires.
    SchedulerTimerFire,
    /// Trigger when one external event wakes the scheduler.
    SchedulerEventWake,
    /// Trigger when time is read.
    TimeRead,
    /// Trigger when random data is read.
    RandomRead,
    /// Trigger when one resource is attached.
    ResourceAttach,
    /// Trigger when one resource is detached.
    ResourceDetach,
}

/// Bounded probability value in parts-per-million.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbabilityPpm(pub u32);

/// Runtime trigger controls.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Trigger {
    /// Hook for this trigger.
    pub on: Hook,
    /// Activation window for this trigger.
    pub activation_window: Option<ActivationWindow>,
    /// Lifetime window for this trigger.
    pub lifetime: Option<Lifetime>,
    /// Activation probability in parts-per-million.
    pub activation_ppm: Option<ProbabilityPpm>,
    /// Fire probability in parts-per-million.
    pub probability_ppm: Option<ProbabilityPpm>,
    /// Maximum number of effect firings.
    pub max_occurrences: Option<u64>,
    /// Cooldown duration between firings in nanoseconds.
    pub cooldown_ns: Option<u64>,
    /// Number of firings per trigger hit.
    pub burst: Option<u32>,
    /// Trigger cadence interval in matching hits.
    pub interval_hits: Option<u64>,
    /// Number of matching hits to skip before cadence starts.
    pub skip_hits: Option<u64>,
}
