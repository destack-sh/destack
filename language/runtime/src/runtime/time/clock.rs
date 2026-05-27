use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::runtime::time::{Instant, Nanos};
use destack_core::{Capture, CaptureMode};
use destack_workspace::{ClockOptions, ExecutionMode};

/// Runtime clock sources and time policies.
#[derive(Debug, Clone)]
pub struct Clock {
    /// Active clock source.
    source: ClockSource,
    /// Runtime-owned clock state.
    runtime_clock: RuntimeClock,
}

impl Clock {
    /// Create a clock from runtime clock options.
    pub fn from_options(
        source: ClockSource,
        options: &ClockOptions,
        default_epoch_nanos: u64,
    ) -> Self {
        let epoch_nanos = match options.epoch_ns {
            Some(epoch_nanos) => epoch_nanos,
            None => default_epoch_nanos,
        };
        let runtime_clock = RuntimeClock::new(epoch_nanos);

        Self {
            source,
            runtime_clock,
        }
    }

    /// Return the active clock source.
    pub const fn source(&self) -> ClockSource {
        self.source
    }

    /// Return the runtime wall time in nanoseconds.
    pub fn wall(&self) -> Nanos {
        self.runtime_clock.wall()
    }

    /// Return the runtime wall time in raw nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the runtime monotonic time in nanoseconds.
    pub fn mono(&self) -> Nanos {
        self.runtime_clock.mono()
    }

    /// Return the runtime monotonic time in raw nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }

    /// Advance the runtime clock to one wall-clock deadline.
    pub fn advance_runtime_to(&self, deadline: Instant) -> Instant {
        self.runtime_clock.advance_to(deadline)
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
        self.runtime_clock
            .restore_snapshot(snapshot.wall, snapshot.monotonic);
    }
}

/// Effective runtime clock source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ClockSource {
    /// Use the host clock directly.
    #[default]
    Host,
    /// Use the runtime-owned clock.
    Runtime,
}

impl ClockSource {
    /// Resolve the effective clock source for one execution mode.
    pub const fn from_execution_mode(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Fast | ExecutionMode::Record => Self::Host,
            ExecutionMode::Strict | ExecutionMode::Replay => Self::Runtime,
        }
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
        Self::from_options(ClockSource::Host, &ClockOptions::default(), 0)
    }
}

/// Runtime-owned clock state.
#[derive(Debug)]
pub(crate) struct RuntimeClock {
    /// Epoch for runtime wall time in nanoseconds.
    epoch_nanos: u64,
    /// Current runtime wall time in nanoseconds.
    wall_nanos: AtomicU64,
    /// Current runtime monotonic time in nanoseconds.
    mono_nanos: AtomicU64,
}

impl RuntimeClock {
    /// Create a new runtime-owned clock.
    pub(crate) fn new(epoch_nanos: u64) -> Self {
        Self {
            epoch_nanos,
            wall_nanos: AtomicU64::new(epoch_nanos),
            mono_nanos: AtomicU64::new(0),
        }
    }

    /// Return the current runtime wall time in nanoseconds.
    pub(crate) fn wall(&self) -> Nanos {
        Nanos::new(self.wall_nanos.load(Ordering::Relaxed))
    }

    /// Return the current runtime monotonic time in nanoseconds.
    pub(crate) fn mono(&self) -> Nanos {
        Nanos::new(self.mono_nanos.load(Ordering::Relaxed))
    }

    /// Advance the runtime clock by a delta.
    pub(crate) fn advance(&self, delta: Nanos) -> Instant {
        let delta_nanos = delta.get();
        let wall = self.wall_nanos.fetch_add(delta_nanos, Ordering::Relaxed) + delta_nanos;
        let _ = self.mono_nanos.fetch_add(delta_nanos, Ordering::Relaxed) + delta_nanos;

        Instant::new(wall)
    }

    /// Advance the runtime clock to the provided deadline.
    pub(crate) fn advance_to(&self, deadline: Instant) -> Instant {
        // keep the current value if we are already past the deadline
        let now = Instant::from_nanos(self.wall());
        if deadline <= now {
            return now;
        }

        // advance by the remaining delta
        let delta = deadline.saturating_sub(now);
        self.advance(delta)
    }

    /// Restore one captured runtime clock state.
    pub(crate) fn restore_snapshot(&self, wall: Instant, mono: Instant) {
        self.wall_nanos.store(wall.get(), Ordering::Relaxed);
        self.mono_nanos.store(mono.get(), Ordering::Relaxed);
    }
}

impl Clone for RuntimeClock {
    fn clone(&self) -> Self {
        Self {
            epoch_nanos: self.epoch_nanos,
            wall_nanos: AtomicU64::new(self.wall().get()),
            mono_nanos: AtomicU64::new(self.mono().get()),
        }
    }
}
