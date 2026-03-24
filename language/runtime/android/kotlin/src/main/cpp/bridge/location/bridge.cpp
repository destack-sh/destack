#include "../types.h"
#include "../jni.h"
#include "runtime.h"

#include <string>

namespace {

/// Build one borrowed native string reference from one Java string.
NativeStringRef string_ref_from_java(
    JNIEnv *env,
    jstring value,
    std::string *storage
) {
    if (value == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    jsize length = env->GetStringUTFLength(value);
    *storage = std::string(chars, chars + length);
    env->ReleaseStringUTFChars(value, chars);

    return NativeStringRef {
        .data = reinterpret_cast<const uint8_t *>(storage->data()),
        .len = static_cast<uint32_t>(storage->size()),
    };
}

}

/// Deliver one location sample into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_location_ProcessLocationAbi_nativeNotifyLocationSample(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jstring watch_id,
    jdouble latitude_degrees,
    jdouble longitude_degrees,
    jdouble altitude_meters,
    jdouble horizontal_accuracy_meters,
    jdouble vertical_accuracy_meters,
    jdouble speed_meters_per_second,
    jdouble heading_degrees,
    jlong timestamp_unix_ns
) {
    std::string watch_id_storage;

    RuntimeStatus status = send_location_sample(
        static_cast<uint64_t>(session_handle),
        string_ref_from_java(env, watch_id, &watch_id_storage),
        LocationSample {
            .latitude_degrees = latitude_degrees,
            .longitude_degrees = longitude_degrees,
            .altitude_meters = altitude_meters,
            .horizontal_accuracy_meters = horizontal_accuracy_meters,
            .vertical_accuracy_meters = vertical_accuracy_meters,
            .speed_meters_per_second = speed_meters_per_second,
            .heading_degrees = heading_degrees,
            .timestamp_unix_ns = static_cast<uint64_t>(timestamp_unix_ns),
        }
    );

    return runtime_status_array(env, status);
}
