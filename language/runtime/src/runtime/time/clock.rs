use std::sync::Arc;

use crate::runtime::time::{HostClock, HostClockSource, VirtualClock};
use destack_workspace::TimeOptions;

/// Default virtual clock tick size in nanoseconds.
const DEFAULT_VIRTUAL_TICK_NS: u64 = 1_000_000;

/// Runtime clock sources and time policies.
#[derive(Debug, Clone)]
pub struct Clock {
    /// Host clock used for wall and monotonic time.
    host_clock: HostClock,
    /// Virtual clock used for deterministic scheduling.
    virtual_clock: VirtualClock,
    /// Time zone identifier for wall time formatting.
    time_zone: Option<String>,
}

impl Clock {
    /// Create a clock from runtime time options.
    pub fn from_options(options: &TimeOptions) -> Self {
        // resolve virtual clock configuration
        let epoch_nanos = options.epoch_ns.unwrap_or(0);
        let tick_nanos = options.tick_ns.unwrap_or(DEFAULT_VIRTUAL_TICK_NS).max(1);
        let virtual_clock = VirtualClock::new(epoch_nanos, tick_nanos);

        Self {
            virtual_clock,
            host_clock: HostClock::new(),
            time_zone: options.time_zone.clone(),
        }
    }

    /// Create a clock from options and one explicit host source.
    pub(crate) fn from_options_with_host_clock_source(
        options: &TimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        // resolve virtual clock configuration
        let epoch_nanos = options.epoch_ns.unwrap_or(0);
        let tick_nanos = options.tick_ns.unwrap_or(DEFAULT_VIRTUAL_TICK_NS).max(1);
        let virtual_clock = VirtualClock::new(epoch_nanos, tick_nanos);

        Self {
            virtual_clock,
            host_clock: HostClock::with_source(host_clock_source),
            time_zone: options.time_zone.clone(),
        }
    }

    /// Return the configured time zone identifier.
    pub fn time_zone(&self) -> Option<&str> {
        self.time_zone.as_deref()
    }

    /// Return the current host wall time in nanoseconds.
    pub fn host_wall_nanos(&self) -> u64 {
        self.host_clock.wall_nanos()
    }

    /// Return the current virtual wall time in nanoseconds.
    pub fn virtual_wall_nanos(&self) -> u64 {
        self.virtual_clock.wall_nanos()
    }

    /// Return the current host monotonic time in nanoseconds.
    pub fn host_mono_nanos(&self) -> u64 {
        self.host_clock.mono_nanos()
    }

    /// Return the current virtual monotonic time in nanoseconds.
    pub fn virtual_mono_nanos(&self) -> u64 {
        self.virtual_clock.mono_nanos()
    }

    /// Advance the virtual clock by a delta.
    pub fn advance_virtual(&self, delta_nanos: u64) -> u64 {
        self.virtual_clock.advance(delta_nanos)
    }

    /// Sleep for one host duration in nanoseconds.
    pub fn host_sleep_nanos(&self, duration_nanos: u64) {
        self.host_clock.sleep_nanos(duration_nanos);
    }

    /// Sleep for one virtual duration in nanoseconds.
    pub fn virtual_sleep_nanos(&self, duration_nanos: u64) {
        let _ = self.virtual_clock.advance(duration_nanos);
    }

    /// Sleep until one host wall deadline in nanoseconds.
    pub fn host_sleep_until_nanos(&self, deadline_nanos: u64) {
        self.host_clock.sleep_until_nanos(deadline_nanos);
    }

    /// Sleep until one virtual wall deadline in nanoseconds.
    pub fn virtual_sleep_until_nanos(&self, deadline_nanos: u64) {
        let _ = self.virtual_clock.advance_to(deadline_nanos);
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::from_options(&TimeOptions::default())
    }
}
