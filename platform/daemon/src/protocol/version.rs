use serde::{Deserialize, Serialize};

/// Current daemon protocol version.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1, 0, 0);
/// Minimum compatible daemon protocol version.
pub const MIN_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1, 0, 0);

/// Version identifier for the daemon protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    /// Major version for breaking changes.
    pub major: u16,
    /// Minor version for backward compatible changes.
    pub minor: u16,
    /// Patch version for fixes and clarifications.
    pub patch: u16,
}

impl ProtocolVersion {
    /// Create a new protocol version.
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Return true when the major version matches.
    pub fn is_compatible_with(self, other: Self) -> bool {
        self.major == other.major
    }
}

/// Protocol version range used for negotiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolRange {
    /// Minimum supported version.
    pub min: ProtocolVersion,
    /// Maximum supported version.
    pub max: ProtocolVersion,
}

impl ProtocolRange {
    /// Create a new protocol range.
    pub const fn new(min: ProtocolVersion, max: ProtocolVersion) -> Self {
        Self { min, max }
    }

    /// Return true when the range includes the given version.
    pub fn contains(&self, version: ProtocolVersion) -> bool {
        version >= self.min && version <= self.max
    }

    /// Return the intersection of two ranges when compatible.
    pub fn intersect(&self, other: &ProtocolRange) -> Option<ProtocolRange> {
        // reject ranges with incompatible major versions
        if !self.min.is_compatible_with(other.min) {
            return None;
        }

        // compute the overlap
        let min = std::cmp::max(self.min, other.min);
        let max = std::cmp::min(self.max, other.max);
        if min > max {
            return None;
        }

        Some(ProtocolRange { min, max })
    }

    /// Select the highest compatible version between two ranges.
    pub fn negotiate(&self, other: &ProtocolRange) -> Option<ProtocolVersion> {
        // compute the overlapping range
        let intersection = self.intersect(other)?;

        // pick the highest compatible version
        Some(intersection.max)
    }
}

#[cfg(test)]
mod tests {
    use super::{ProtocolRange, ProtocolVersion};

    #[test]
    fn test_protocol_range_intersection_selects_highest() {
        // build overlapping ranges
        let a = ProtocolRange::new(ProtocolVersion::new(1, 0, 0), ProtocolVersion::new(1, 4, 0));
        let b = ProtocolRange::new(ProtocolVersion::new(1, 2, 0), ProtocolVersion::new(1, 3, 0));

        // assert the negotiated version
        let negotiated = a.negotiate(&b);
        assert_eq!(negotiated, Some(ProtocolVersion::new(1, 3, 0)));
    }

    #[test]
    fn test_protocol_range_rejects_incompatible_major() {
        // build incompatible ranges
        let a = ProtocolRange::new(ProtocolVersion::new(1, 0, 0), ProtocolVersion::new(1, 1, 0));
        let b = ProtocolRange::new(ProtocolVersion::new(2, 0, 0), ProtocolVersion::new(2, 1, 0));

        // assert negotiation fails
        let negotiated = a.negotiate(&b);
        assert_eq!(negotiated, None);
    }
}
