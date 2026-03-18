use objc2_core_location::{CLHeading, CLLocation};

use crate::platform::os::abi_generated::LocationSampleValue;

/// Map one native location sample into one runtime sample payload.
pub(super) fn location_sample_value_from_native(
    location: &CLLocation,
    heading_override: Option<f64>,
) -> LocationSampleValue {
    let coordinate = unsafe { location.coordinate() };
    let altitude_meters = optional_finite(unsafe { location.altitude() });
    let horizontal_accuracy_meters = normalize_accuracy(unsafe { location.horizontalAccuracy() });
    let vertical_accuracy_meters = normalize_accuracy(unsafe { location.verticalAccuracy() });
    let speed_meters_per_second = normalize_speed(unsafe { location.speed() });
    let heading_degrees =
        heading_override.or_else(|| normalize_heading(unsafe { location.course() }));
    let timestamp_seconds = unsafe { location.timestamp() }.timeIntervalSince1970();
    let timestamp_unix_ns = unix_nanoseconds_from_seconds(timestamp_seconds);

    LocationSampleValue {
        latitude_degrees: coordinate.latitude,
        longitude_degrees: coordinate.longitude,
        altitude_meters,
        horizontal_accuracy_meters,
        vertical_accuracy_meters,
        speed_meters_per_second,
        heading_degrees,
        timestamp_unix_ns,
    }
}

/// Map one native heading payload into one runtime heading value.
pub(super) fn heading_degrees_from_native(heading: &CLHeading) -> Option<f64> {
    let true_heading = unsafe { heading.trueHeading() };
    if true_heading >= 0.0 {
        return Some(true_heading);
    }

    normalize_heading(unsafe { heading.magneticHeading() })
}

/// Return one optional finite scalar.
fn optional_finite(value: f64) -> Option<f64> {
    if value.is_finite() {
        return Some(value);
    }

    None
}

/// Normalize one accuracy payload that uses negative values for absence.
fn normalize_accuracy(accuracy: f64) -> Option<f64> {
    if accuracy.is_finite() && accuracy >= 0.0 {
        return Some(accuracy);
    }

    None
}

/// Normalize one speed payload that uses negative values for absence.
fn normalize_speed(speed: f64) -> Option<f64> {
    if speed.is_finite() && speed >= 0.0 {
        return Some(speed);
    }

    None
}

/// Normalize one heading payload that uses negative values for absence.
fn normalize_heading(heading: f64) -> Option<f64> {
    if heading.is_finite() && heading >= 0.0 {
        return Some(heading);
    }

    None
}

/// Convert one unix-seconds payload into nanoseconds with saturation.
fn unix_nanoseconds_from_seconds(seconds: f64) -> u64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0;
    }

    let nanoseconds = seconds * 1_000_000_000.0;

    nanoseconds.clamp(0.0, u64::MAX as f64) as u64
}
