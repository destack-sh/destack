#[cfg(feature = "generator")]
use crate::host::abi::describe::host_abi_types;

#[allow(unused_imports)]
#[cfg(not(feature = "generator"))]
pub(crate) use crate::platform::os::{LocationAccuracy, LocationSample, LocationWatchOptions};

/// One location-services response payload.
#[cfg(not(feature = "generator"))]
#[derive(Clone, Debug, PartialEq)]
pub struct LocationServicesResponse {
    /// The request status code.
    pub status: u32,
    /// Whether location services are enabled.
    pub is_enabled: bool,
}

#[cfg(not(feature = "generator"))]
impl LocationServicesResponse {
    /// Create one location-services response payload.
    pub const fn new(status: u32, is_enabled: bool) -> Self {
        Self { status, is_enabled }
    }
}

/// One last-known location response payload.
#[cfg(not(feature = "generator"))]
#[derive(Clone, Debug, PartialEq)]
pub struct LocationLastKnownResponse {
    /// The request status code.
    pub status: u32,
    /// The returned sample when available.
    pub sample: Option<LocationSample>,
}

#[cfg(not(feature = "generator"))]
impl LocationLastKnownResponse {
    /// Create one last-known location response payload.
    pub const fn new(status: u32, sample: Option<LocationSample>) -> Self {
        Self { status, sample }
    }
}

#[cfg(feature = "generator")]
host_abi_types! {
    fn host_abi_types() {
        /// One location accuracy preference.
        enum LocationAccuracy: i32 {
            /// Passive.
            Passive = 1,
            /// Low.
            Low = 2,
            /// Balanced.
            Balanced = 3,
            /// High.
            High = 4,
            /// Best.
            Best = 5,
        }

        /// One location sample payload.
        struct LocationSample {
            /// Latitude in degrees.
            latitude_degrees: f64,
            /// Longitude in degrees.
            longitude_degrees: f64,
            /// Altitude in meters above mean sea level when available.
            altitude_meters: f64,
            /// Horizontal accuracy radius in meters.
            horizontal_accuracy_meters: f64,
            /// Vertical accuracy in meters when available.
            vertical_accuracy_meters: f64,
            /// Speed in meters per second when available.
            speed_meters_per_second: f64,
            /// Heading in degrees when available.
            heading_degrees: f64,
            /// UTC timestamp in nanoseconds.
            timestamp_unix_ns: u64,
        }

        /// One location watch-options payload.
        struct LocationWatchOptions {
            /// Accuracy preference.
            accuracy: LocationAccuracy,
            /// Minimum interval between updates in nanoseconds.
            minimum_interval_ns: u64,
            /// Minimum distance delta in meters.
            minimum_distance_meters: f64,
            /// Whether heading should be included when available.
            include_heading: bool,
        }

        /// One location-services response payload.
        struct LocationServicesResponse {
            /// The request status code.
            status: host_status,
            /// Whether location services are enabled.
            is_enabled: bool,
        }

        /// One last-known location response payload.
        struct LocationLastKnownResponse {
            /// The request status code.
            status: host_status,
            /// Whether the response includes one sample.
            has_sample: bool,
            /// The returned sample when available.
            sample: LocationSample,
        }
    }
}
