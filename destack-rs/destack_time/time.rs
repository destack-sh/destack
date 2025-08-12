/// Time in unsigned 64-bit nanosecond precision
/// Range: 00:00:00.000000000 to 23:59:59.999999999
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(u64);

impl Time {
    /// Get the current Time.
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime before UNIX_EPOCH");
        // get nanoseconds since midnight today
        let nanos_since_midnight = now.as_nanos() % (24 * 60 * 60 * 1_000_000_000);
        Self(nanos_since_midnight as u64)
    }
}


