/// Policy for accessing wall clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallClockPolicy {
    /// Wall clock reads are forbidden.
    Disabled,
    /// Wall clock reads are allowed but logged.
    #[default]
    Logged,
    /// Wall clock reads pass through without logging.
    Passthrough,
}
