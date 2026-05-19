use crate::diagnostic::RuntimeResult;
use crate::host::Host;
use crate::host::random::HostRandom;
use crate::host::time::{HostClock, HostClockSource};
use std::sync::Arc;

/// Windows host integration.
#[derive(Debug)]
pub(crate) struct WindowsHost {
    /// Host wall and monotonic clock provider.
    clock: HostClock,
    /// Host entropy provider.
    random: HostRandom,
}

impl WindowsHost {
    /// Create one Windows host integration.
    pub(crate) fn new(clock_source: Option<Arc<dyn HostClockSource>>) -> Self {
        let clock = match clock_source {
            Some(clock_source) => HostClock::with_source(clock_source),
            None => HostClock::new(),
        };

        Self {
            clock,
            random: HostRandom::new(),
        }
    }
}

impl Host for WindowsHost {
    fn wall_nanos(&self) -> u64 {
        self.clock.wall_nanos()
    }

    fn mono_nanos(&self) -> u64 {
        self.clock.mono_nanos()
    }

    fn sleep_nanos(&self, duration_nanos: u64) {
        self.clock.sleep_nanos(duration_nanos);
    }

    fn sleep_until_wall_nanos(&self, deadline_nanos: u64) {
        self.clock.sleep_until_nanos(deadline_nanos);
    }

    fn fill_random_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        self.random.fill_bytes(buffer)
    }

    fn try_fill_random_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        self.random.try_fill_bytes(buffer)
    }

    fn random_u64(&self) -> RuntimeResult<u64> {
        self.random.next_u64()
    }
}
