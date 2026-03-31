#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_TYPES_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_TYPES_H

#include "primitives.h"
#include "types.generated.h"

/// One owned native array passed through the Android bridge.
template <typename T>
struct NativeArray {
    /// The array data pointer.
    T *data;
    /// The logical length in elements.
    uint32_t len;
    /// The allocated capacity in elements.
    uint32_t capacity;
};

/// One optional string reference passed through the Android bridge.
struct HostOptionalStringRef {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped string reference.
    NativeStringRef value;
};

/// One optional `u32` passed through the Android bridge.
struct HostOptionalU32 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    uint32_t value;
};

/// One optional `u64` passed through the Android bridge.
struct HostOptionalU64 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    uint64_t value;
};

/// One optional `i8` passed through the Android bridge.
struct HostOptionalI8 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    int8_t value;
};

#endif
