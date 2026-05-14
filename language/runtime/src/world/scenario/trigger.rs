use serde::{Deserialize, Serialize};

use super::Hook;

/// Runtime trigger controls.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Trigger {
    /// Hook for this trigger.
    pub on: Hook,
    /// Activation window for this trigger.
    pub activation: Option<ActivationWindow>,
    /// Lifetime window for this trigger.
    pub lifetime: Option<Lifetime>,
    /// Activation probability in parts-per-million.
    pub activation_ppm: Option<ProbabilityPpm>,
    /// Fire probability in parts-per-million.
    pub probability_ppm: Option<ProbabilityPpm>,
    /// Maximum number of action firings.
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

impl Trigger {
    /// Create one trigger for one hook with default gates.
    pub fn on(hook: Hook) -> Self {
        Self {
            on: hook,
            activation: None,
            lifetime: None,
            activation_ppm: None,
            probability_ppm: None,
            max_occurrences: None,
            cooldown_ns: None,
            burst: None,
            interval_hits: None,
            skip_hits: None,
        }
    }

    /// Create one trigger that fires on every matching hook event.
    pub fn always(hook: Hook) -> Self {
        Self::on(hook)
    }

    /// Create one trigger that fires once on the first matching hook event.
    pub fn once(hook: Hook) -> Self {
        Self::on(hook).max_occurrences(1)
    }

    /// Create one trigger that fires once per N matching hook hits.
    pub fn every_hits(hook: Hook, interval_hits: u64) -> Self {
        Self::on(hook).interval_hits(interval_hits)
    }

    /// Create one trigger with one fire probability in parts-per-million.
    pub fn with_probability(hook: Hook, probability_ppm: u32) -> Self {
        Self::on(hook).probability_ppm(ProbabilityPpm::new(probability_ppm))
    }

    /// Set activation window.
    pub fn activation(mut self, activation: ActivationWindow) -> Self {
        self.activation = Some(activation);
        self
    }

    /// Set lifetime window.
    pub fn lifetime(mut self, lifetime: Lifetime) -> Self {
        self.lifetime = Some(lifetime);
        self
    }

    /// Set activation probability.
    pub fn activation_ppm(mut self, probability: ProbabilityPpm) -> Self {
        self.activation_ppm = Some(probability);
        self
    }

    /// Set fire probability.
    pub fn probability_ppm(mut self, probability: ProbabilityPpm) -> Self {
        self.probability_ppm = Some(probability);
        self
    }

    /// Set maximum occurrences.
    pub fn max_occurrences(mut self, max_occurrences: u64) -> Self {
        self.max_occurrences = Some(max_occurrences);
        self
    }

    /// Set cooldown duration in nanoseconds.
    pub fn cooldown_ns(mut self, cooldown_ns: u64) -> Self {
        self.cooldown_ns = Some(cooldown_ns);
        self
    }

    /// Set burst count.
    pub fn burst(mut self, burst: u32) -> Self {
        self.burst = Some(burst);
        self
    }

    /// Set cadence interval in matching hits.
    pub fn interval_hits(mut self, interval_hits: u64) -> Self {
        self.interval_hits = Some(interval_hits);
        self
    }

    /// Set number of matching hits to skip before cadence starts.
    pub fn skip_hits(mut self, skip_hits: u64) -> Self {
        self.skip_hits = Some(skip_hits);
        self
    }
}

/// Activation window for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivationWindow {
    /// Activate the rule immediately.
    Immediate,
    /// Activate once monotonic scenario time reaches this timestamp.
    AtTimeNs {
        /// Activation timestamp in monotonic nanoseconds.
        time_ns: u64,
    },
    /// Activate once this number of matching calls has elapsed.
    AfterCallCount {
        /// Matching call count before activation.
        call_count: u64,
    },
}

impl ActivationWindow {
    /// Return true when this activation window is reached.
    pub fn is_reached(&self, total_calls_seen: u64, now_ns: u64) -> bool {
        match self {
            Self::Immediate => true,
            Self::AtTimeNs { time_ns } => now_ns >= *time_ns,
            Self::AfterCallCount { call_count } => total_calls_seen >= *call_count,
        }
    }
}

/// Lifetime window for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lifetime {
    /// Keep the rule active until explicitly disabled.
    UntilDisabled,
    /// Keep the rule active for this monotonic duration.
    ForDurationNs {
        /// Active duration in nanoseconds.
        duration_ns: u64,
    },
    /// Keep the rule active for this number of matching calls.
    ForCallCount {
        /// Active call budget before expiration.
        call_count: u64,
    },
}

/// Bounded probability value in parts-per-million.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbabilityPpm(pub u32);

impl ProbabilityPpm {
    /// Create one parts-per-million probability value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}
