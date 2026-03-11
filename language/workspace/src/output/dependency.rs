/// Dependency stamp captured for one output build.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct OutputDependency(pub u64);

impl std::fmt::Debug for OutputDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl std::fmt::Display for OutputDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl OutputDependency {
    /// Create a new output dependency.
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}
