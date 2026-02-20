use crate::runtime::time::{HostClock, VirtualClock, WallClockPolicy};
use destack_workspace::{TimeMode, TimeOptions};

/// Default virtual clock tick size in nanoseconds.
const DEFAULT_VIRTUAL_TICK_NS: u64 = 1_000_000;

// TODO #Incomplete: enforce wall clock policy in runtime host time adapters
// TODO #Incomplete: apply the time zone to wall clock formatting helpers

/// Runtime clock sources and time policies.
#[derive(Debug, Clone)]
pub struct Clock {
    /// Active time mode for the runtime.
    mode: TimeMode,
    /// Virtual clock used for deterministic scheduling.
    virtual_clock: VirtualClock,
    /// Host clock used for wall and monotonic time.
    host_clock: HostClock,
    /// Wall clock policy used for host time access.
    wall_clock_policy: WallClockPolicy,
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

        // select the wall clock policy
        let wall_clock_policy = match options.mode {
            TimeMode::Host => WallClockPolicy::Passthrough,
            TimeMode::Virtual => WallClockPolicy::Logged,
        };

        Self {
            mode: options.mode,
            virtual_clock,
            host_clock: HostClock::new(),
            wall_clock_policy,
            time_zone: options.time_zone.clone(),
        }
    }

    /// Return the active time mode.
    pub fn mode(&self) -> TimeMode {
        self.mode
    }

    /// Return the wall clock policy.
    pub fn wall_clock_policy(&self) -> WallClockPolicy {
        self.wall_clock_policy
    }

    /// Return the configured time zone identifier.
    pub fn time_zone(&self) -> Option<&str> {
        self.time_zone.as_deref()
    }

    /// Return the current wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        // select the wall clock source
        match self.mode {
            TimeMode::Host => self.host_clock.wall_nanos(),
            TimeMode::Virtual => self.virtual_clock.wall_nanos(),
        }
    }

    /// Return the current monotonic time in nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        // select the monotonic clock source
        match self.mode {
            TimeMode::Host => self.host_clock.mono_nanos(),
            TimeMode::Virtual => self.virtual_clock.mono_nanos(),
        }
    }

    /// Advance the virtual clock by a delta.
    pub fn advance_virtual(&self, delta_nanos: u64) -> u64 {
        self.virtual_clock.advance(delta_nanos)
    }

    /// Sleep for the given duration in nanoseconds.
    pub fn sleep_nanos(&self, duration_nanos: u64) {
        // route sleep based on mode
        match self.mode {
            TimeMode::Host => self.host_clock.sleep_nanos(duration_nanos),
            TimeMode::Virtual => {
                let _ = self.virtual_clock.advance(duration_nanos);
            }
        }
    }

    /// Sleep until the provided deadline in nanoseconds.
    pub fn sleep_until_nanos(&self, deadline_nanos: u64) {
        // route sleep based on mode
        match self.mode {
            TimeMode::Host => self.host_clock.sleep_until_nanos(deadline_nanos),
            TimeMode::Virtual => {
                let _ = self.virtual_clock.advance_to(deadline_nanos);
            }
        }
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::from_options(&TimeOptions::default())
    }
}
