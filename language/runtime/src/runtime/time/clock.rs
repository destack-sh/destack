use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::host::time::{HostClock, HostClockSource};
use crate::runtime::time::{Instant, Nanos, VirtualClock};
use destack_core::{Capture, CaptureMode};
use destack_workspace::TimeOptions;

/// Default virtual clock epoch.
const DEFAULT_TIME_EPOCH_NANOS: u64 = 0;

/// Materialized clock state captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockImage {
    /// Captured virtual wall-clock instant.
    pub virtual_wall: Instant,
    /// Captured virtual monotonic instant.
    pub virtual_mono: Instant,
}

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
        let epoch_nanos = match options.epoch_ns {
            Some(epoch_nanos) => epoch_nanos,
            None => DEFAULT_TIME_EPOCH_NANOS,
        };
        let virtual_clock = VirtualClock::new(epoch_nanos);

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
        let epoch_nanos = match options.epoch_ns {
            Some(epoch_nanos) => epoch_nanos,
            None => DEFAULT_TIME_EPOCH_NANOS,
        };
        let virtual_clock = VirtualClock::new(epoch_nanos);

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
    pub fn host_wall(&self) -> Nanos {
        Nanos::new(self.host_clock.wall_nanos())
    }

    /// Return the current host wall time in nanoseconds.
    pub fn host_wall_nanos(&self) -> u64 {
        self.host_wall().get()
    }

    /// Return the current virtual wall time in nanoseconds.
    pub fn virtual_wall(&self) -> Nanos {
        self.virtual_clock.wall()
    }

    /// Return the current virtual wall time in nanoseconds.
    pub fn virtual_wall_nanos(&self) -> u64 {
        self.virtual_wall().get()
    }

    /// Return the current host monotonic time in nanoseconds.
    pub fn host_mono(&self) -> Nanos {
        Nanos::new(self.host_clock.mono_nanos())
    }

    /// Return the current host monotonic time in nanoseconds.
    pub fn host_mono_nanos(&self) -> u64 {
        self.host_mono().get()
    }

    /// Return the current virtual monotonic time in nanoseconds.
    pub fn virtual_mono(&self) -> Nanos {
        self.virtual_clock.mono()
    }

    /// Return the current virtual monotonic time in nanoseconds.
    pub fn virtual_mono_nanos(&self) -> u64 {
        self.virtual_mono().get()
    }

    /// Advance the virtual clock to one wall-clock deadline.
    pub fn advance_virtual_to(&self, deadline: Instant) -> Instant {
        self.virtual_clock.advance_to(deadline)
    }

    /// Capture one materialized clock image.
    pub(crate) fn snapshot(&self) -> ClockImage {
        ClockImage {
            virtual_wall: Instant::from_nanos(self.virtual_wall()),
            virtual_mono: Instant::from_nanos(self.virtual_mono()),
        }
    }

    /// Restore one materialized clock image.
    pub(crate) fn restore_snapshot(&self, snapshot: &ClockImage) {
        self.virtual_clock
            .restore_snapshot(snapshot.virtual_wall, snapshot.virtual_mono);
    }

    /// Sleep for one host duration in nanoseconds.
    pub fn host_sleep_nanos(&self, duration_nanos: u64) {
        self.host_clock.sleep_nanos(duration_nanos);
    }

    /// Sleep until one host wall deadline in nanoseconds.
    pub fn host_sleep_until_nanos(&self, deadline_nanos: u64) {
        self.host_clock.sleep_until_nanos(deadline_nanos);
    }
}

impl Capture for Clock {
    type Image = ClockImage;
    type Error = std::convert::Infallible;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one clock image.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Ok(self.snapshot())
    }

    /// Restore one clock image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_snapshot(image);

        Ok(())
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::from_options(&TimeOptions::default())
    }
}
