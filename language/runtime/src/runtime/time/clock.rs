use serde::{Deserialize, Serialize};

use crate::runtime::time::{Instant, Nanos, VirtualClock};
use destack_core::{Capture, CaptureMode};
use destack_workspace::{ClockSource, TimeOptions};

/// Default virtual clock epoch.
const DEFAULT_TIME_EPOCH_NANOS: u64 = 0;

/// Runtime clock sources and time policies.
#[derive(Debug, Clone)]
pub struct Clock {
    /// Active clock source.
    source: ClockSource,
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
            source: options.source,
            virtual_clock,
            time_zone: options.time_zone.clone(),
        }
    }

    /// Return the active clock source.
    pub const fn source(&self) -> ClockSource {
        self.source
    }

    /// Return the configured time zone identifier.
    pub fn time_zone(&self) -> Option<&str> {
        self.time_zone.as_deref()
    }

    /// Return the virtual wall time in nanoseconds.
    pub fn wall(&self) -> Nanos {
        self.virtual_clock.wall()
    }

    /// Return the virtual wall time in raw nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the virtual monotonic time in nanoseconds.
    pub fn mono(&self) -> Nanos {
        self.virtual_clock.mono()
    }

    /// Return the virtual monotonic time in raw nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }

    /// Advance the virtual clock to one wall-clock deadline.
    pub fn advance_virtual_to(&self, deadline: Instant) -> Instant {
        self.virtual_clock.advance_to(deadline)
    }

    /// Capture one materialized clock image.
    pub(crate) fn snapshot(&self) -> ClockImage {
        ClockImage {
            wall: Instant::from_nanos(self.wall()),
            monotonic: Instant::from_nanos(self.mono()),
        }
    }

    /// Restore one materialized clock image.
    pub(crate) fn restore_snapshot(&self, snapshot: &ClockImage) {
        self.virtual_clock
            .restore_snapshot(snapshot.wall, snapshot.monotonic);
    }
}

/// Materialized clock state captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockImage {
    /// Captured wall-clock instant.
    pub wall: Instant,
    /// Captured monotonic instant.
    pub monotonic: Instant,
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
