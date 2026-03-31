#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_PRIMITIVES_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_PRIMITIVES_H

#include <jni.h>

#include <stdint.h>

/// The host status code for one successful bridge call.
constexpr uint32_t HOST_STATUS_OK = 0;
/// The host status code for one missing bridge binding.
constexpr uint32_t HOST_STATUS_NOT_FOUND = 3;
/// The host status code for one buffer that was too small.
constexpr uint32_t HOST_STATUS_BUFFER_TOO_SMALL = 5;
/// The host status code for one generic bridge failure.
constexpr uint32_t HOST_STATUS_FAILED = 6;

/// One mutable byte slice passed through the Android bridge.
struct NativeSlice {
    /// The slice data pointer.
    uint8_t *data;
    /// The slice length in bytes.
    uint32_t len;
};

/// One borrowed string reference passed through the Android bridge.
struct NativeStringRef {
    /// The string data pointer.
    const uint8_t *data;
    /// The string length in bytes.
    uint32_t len;
};

/// One borrowed string-slice reference passed through the Android bridge.
struct NativeStringSlice {
    /// The slice data pointer.
    const NativeStringRef *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One `u8` slice passed through the Android bridge.
struct NativeU8Slice {
    /// The slice data pointer.
    const uint8_t *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One `i8` slice passed through the Android bridge.
struct NativeI8Slice {
    /// The slice data pointer.
    const int8_t *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One `i16` slice passed through the Android bridge.
struct NativeI16Slice {
    /// The slice data pointer.
    const int16_t *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One runtime status returned by one bridge ingress call.
struct RuntimeStatus {
    /// The status code.
    uint32_t code;
    /// The optional runtime error identifier.
    uint64_t error_id;
};

#endif
