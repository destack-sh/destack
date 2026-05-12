#[allow(unused_imports)]
pub(crate) use crate::platform::os::{LocationAccuracy, LocationSample, LocationWatchOptions};

/// One location-services response payload.
#[derive(Clone, Debug, PartialEq)]
pub struct LocationServicesResponse {
    /// The request status code.
    pub status: u32,
    /// Whether location services are enabled.
    pub is_enabled: bool,
}

impl LocationServicesResponse {
    /// Create one location-services response payload.
    pub const fn new(status: u32, is_enabled: bool) -> Self {
        Self { status, is_enabled }
    }
}

/// One last-known location response payload.
#[derive(Clone, Debug, PartialEq)]
pub struct LocationLastKnownResponse {
    /// The request status code.
    pub status: u32,
    /// The returned sample when available.
    pub sample: Option<LocationSample>,
}

impl LocationLastKnownResponse {
    /// Create one last-known location response payload.
    pub const fn new(status: u32, sample: Option<LocationSample>) -> Self {
        Self { status, sample }
    }
}
