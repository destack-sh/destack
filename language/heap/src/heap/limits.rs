use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

/// Hard limits for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapLimits {
    /// Optional hard limit for total live heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limits for the managed space.
    pub managed: ManagedLimits,
    /// Hard limits for the raw space.
    pub raw: RawLimits,
}

/// Hard limits for one live managed space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManagedLimits {
    /// Optional hard limit for active managed heap bytes.
    pub max_bytes: Option<u64>,
}

/// Hard limits for one live raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawLimits {
    /// Optional hard limit for active raw heap bytes.
    pub max_bytes: Option<u64>,
}

impl HeapLimits {
    /// Check exact managed and raw active bytes against these limits.
    pub fn check(&self, managed_bytes: u64, raw_bytes: u64) -> Result<(), HeapLimitError> {
        // check managed heap limit
        if let Some(max_bytes) = self.managed.max_bytes
            && managed_bytes > max_bytes
        {
            return Err(HeapLimitError {
                scope: HeapLimitScope::Managed,
                used_bytes: managed_bytes,
                max_bytes,
            });
        }

        // check raw heap limit
        if let Some(max_bytes) = self.raw.max_bytes
            && raw_bytes > max_bytes
        {
            return Err(HeapLimitError {
                scope: HeapLimitScope::Raw,
                used_bytes: raw_bytes,
                max_bytes,
            });
        }

        // check total heap limit
        let total_bytes = managed_bytes.saturating_add(raw_bytes);
        if let Some(max_bytes) = self.max_bytes
            && total_bytes > max_bytes
        {
            return Err(HeapLimitError {
                scope: HeapLimitScope::Total,
                used_bytes: total_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check exact managed and raw active bytes after one requested reservation.
    pub fn check_active_reservation(
        &self,
        managed_bytes: u64,
        raw_bytes: u64,
        managed_reservation: i64,
        raw_reservation: i64,
    ) -> Result<(), HeapLimitError> {
        let managed_bytes = apply_reservation(managed_bytes, managed_reservation);
        let raw_bytes = apply_reservation(raw_bytes, raw_reservation);

        self.check(managed_bytes, raw_bytes)
    }
}

/// Apply one active-byte reservation to one current byte count.
fn apply_reservation(current: u64, reservation: i64) -> u64 {
    // positive reservations grow the current active bytes
    if reservation >= 0 {
        current.saturating_add(reservation as u64)
    }
    // negative reservations release active bytes
    else {
        current.saturating_sub(reservation.unsigned_abs())
    }
}

/// The heap space that exceeded one configured hard limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapLimitScope {
    /// The total live heap exceeded its limit.
    Total,
    /// The managed live heap exceeded its limit.
    Managed,
    /// The raw live heap exceeded its limit.
    Raw,
}

impl HeapLimitScope {
    /// Return the display name for this heap limit scope.
    pub const fn name(self) -> &'static str {
        // each scope exposes one stable diagnostic label
        match self {
            Self::Total => "total",
            Self::Managed => "managed",
            Self::Raw => "raw",
        }
    }
}

/// One heap hard-limit violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeapLimitError {
    /// The heap space that exceeded its hard limit.
    pub scope: HeapLimitScope,
    /// The exact live bytes in that heap space.
    pub used_bytes: u64,
    /// The configured hard limit in bytes.
    pub max_bytes: u64,
}

impl fmt::Display for HeapLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} heap limit exceeded: using {} bytes with limit {}",
            self.scope.name(),
            self.used_bytes,
            self.max_bytes,
        )
    }
}

impl Error for HeapLimitError {}
