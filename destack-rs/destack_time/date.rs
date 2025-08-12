/// Date in signed 64-bit day precision since epoch (UTC).
/// Range: ±2.525x10^16 days
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(i64);

impl Date {
    /// Get the current Date.
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
                .as_secs() as i64
                / 86400, // convert seconds to days
        )
    }
}


