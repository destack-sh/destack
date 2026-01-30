/// Phase of the garbage collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcPhase {
    /// GC is idle.
    Idle,
    /// GC is marking reachable objects.
    Mark,
    /// GC is draining remaining work and finalizing the mark phase.
    MarkTermination,
    /// GC is sweeping unreachable objects.
    Sweep,
}

impl GcPhase {
    /// Report whether the GC is currently marking.
    pub fn is_marking(self) -> bool {
        matches!(self, Self::Mark | Self::MarkTermination)
    }
}
