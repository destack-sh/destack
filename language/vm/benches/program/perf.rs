#[cfg(all(feature = "perf-counters", target_os = "linux"))]
mod imp {
    use perf_event::Builder;
    use perf_event::events::{CacheId, CacheOpId, CacheOpResultId, Hardware, HardwareCache};

    /// Hardware performance counters collected during a run.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct PerfReport {
        /// Branch miss count.
        pub branch_misses: u64,
        /// L1 data cache miss count.
        pub l1_misses: u64,
        /// L1 data cache access count.
        pub l1_accesses: u64,
    }

    impl PerfReport {
        /// Return the l1 miss rate as a fraction.
        #[allow(dead_code)]
        pub(crate) fn l1_miss_rate(self) -> Option<f64> {
            if self.l1_accesses == 0 {
                return None;
            }

            Some(self.l1_misses as f64 / self.l1_accesses as f64)
        }
    }

    /// Perf counter wrapper for benchmark runs.
    pub(crate) struct PerfCounters {
        enabled: bool,
        inner: Option<PerfCountersLinux>,
    }

    struct PerfCountersLinux {
        branch_misses: perf_event::Counter,
        l1_accesses: perf_event::Counter,
        l1_misses: perf_event::Counter,
    }

    impl PerfCountersLinux {
        fn new() -> Option<Self> {
            let branch_misses = Builder::new().kind(Hardware::BRANCH_MISSES).build().ok()?;
            let l1_accesses = Builder::new()
                .kind(HardwareCache::new(
                    CacheId::L1D,
                    CacheOpId::READ,
                    CacheOpResultId::ACCESS,
                ))
                .build()
                .ok()?;
            let l1_misses = Builder::new()
                .kind(HardwareCache::new(
                    CacheId::L1D,
                    CacheOpId::READ,
                    CacheOpResultId::MISS,
                ))
                .build()
                .ok()?;

            Some(Self {
                branch_misses,
                l1_accesses,
                l1_misses,
            })
        }

        fn start(&mut self) {
            let _ = self.branch_misses.reset();
            let _ = self.l1_accesses.reset();
            let _ = self.l1_misses.reset();

            let _ = self.branch_misses.enable();
            let _ = self.l1_accesses.enable();
            let _ = self.l1_misses.enable();
        }

        fn stop(&mut self) -> Option<PerfReport> {
            let _ = self.branch_misses.disable();
            let _ = self.l1_accesses.disable();
            let _ = self.l1_misses.disable();

            let branch_misses = self.branch_misses.read().ok()?;
            let l1_accesses = self.l1_accesses.read().ok()?;
            let l1_misses = self.l1_misses.read().ok()?;

            Some(PerfReport {
                branch_misses,
                l1_accesses,
                l1_misses,
            })
        }
    }

    impl PerfCounters {
        /// Create a perf counter collection handle.
        pub(crate) fn new(enabled: bool) -> Self {
            let inner = if enabled {
                PerfCountersLinux::new()
            } else {
                None
            };
            Self { enabled, inner }
        }

        /// Check if perf counters are available.
        pub(crate) fn is_supported(&self) -> bool {
            self.enabled && self.inner.is_some()
        }

        /// Start counter collection.
        pub(crate) fn start(&mut self) {
            if !self.enabled {
                return;
            }

            if let Some(inner) = self.inner.as_mut() {
                inner.start();
            }
        }

        /// Stop counter collection and return results.
        pub(crate) fn stop(&mut self) -> Option<PerfReport> {
            if !self.enabled {
                return None;
            }

            self.inner.as_mut()?.stop()
        }
    }
}

#[cfg(not(all(feature = "perf-counters", target_os = "linux")))]
mod imp {
    /// Hardware performance counters collected during a run.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct PerfReport {
        /// Branch miss count.
        pub branch_misses: u64,
        /// L1 data cache miss count.
        pub l1_misses: u64,
        /// L1 data cache access count.
        pub l1_accesses: u64,
    }

    impl PerfReport {
        /// Return the l1 miss rate as a fraction.
        #[allow(dead_code)]
        pub(crate) fn l1_miss_rate(self) -> Option<f64> {
            if self.l1_accesses == 0 {
                return None;
            }

            Some(self.l1_misses as f64 / self.l1_accesses as f64)
        }
    }

    /// Perf counter wrapper for benchmark runs.
    pub(crate) struct PerfCounters {
        enabled: bool,
    }

    impl PerfCounters {
        /// Create a perf counter collection handle.
        pub(crate) fn new(enabled: bool) -> Self {
            Self { enabled }
        }

        /// Check if perf counters are available.
        pub(crate) fn is_supported(&self) -> bool {
            self.enabled && false
        }

        /// Start counter collection.
        pub(crate) fn start(&mut self) {}

        /// Stop counter collection and return results.
        pub(crate) fn stop(&mut self) -> Option<PerfReport> {
            None
        }
    }
}

pub(crate) use imp::PerfCounters;
