#include "../types.h"
#include "loader.h"

#include <string>

namespace {

using TestLocationServicesEnabledFunction = uint32_t (*)(uint64_t, bool *);
using TestLocationWatchOpenFunction =
    uint32_t (*)(uint64_t, NativeStringRef, LocationWatchOptions);
using TestLocationWatchCloseFunction = uint32_t (*)(uint64_t, NativeStringRef);

TestLocationServicesEnabledFunction test_location_services_enabled = nullptr;
TestLocationWatchOpenFunction test_location_watch_open = nullptr;
TestLocationWatchCloseFunction test_location_watch_close = nullptr;

// resolve the location bridge symbols
bool resolve_location_testing_symbols() {
    if (test_location_services_enabled == nullptr) {
        test_location_services_enabled = reinterpret_cast<TestLocationServicesEnabledFunction>(
            resolve_testing_symbol("destack_host_android_location_services_enabled")
        );
    }

    if (test_location_watch_open == nullptr) {
        test_location_watch_open = reinterpret_cast<TestLocationWatchOpenFunction>(
            resolve_testing_symbol("destack_host_android_location_watch_open")
        );
    }

    if (test_location_watch_close == nullptr) {
        test_location_watch_close = reinterpret_cast<TestLocationWatchCloseFunction>(
            resolve_testing_symbol("destack_host_android_location_watch_close")
        );
    }

    return
        test_location_services_enabled != nullptr &&
        test_location_watch_open != nullptr &&
        test_location_watch_close != nullptr;
}

}

/// Query location-services-enabled through the Android bridge test harness.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeTestLocationServicesEnabled(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle
) {
    if (!resolve_location_testing_symbols()) {
        return nullptr;
    }

    bool is_enabled = false;
    uint32_t status = test_location_services_enabled(
        static_cast<uint64_t>(session_handle),
        &is_enabled
    );

    jlong values[2] = {
        static_cast<jlong>(status),
        static_cast<jlong>(is_enabled ? 1 : 0),
    };
    jlongArray result = env->NewLongArray(2);
    if (result == nullptr) {
        return nullptr;
    }

    env->SetLongArrayRegion(result, 0, 2, values);

    return result;
}

/// Open one location watch through the Android bridge test harness.
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeTestLocationWatchOpen(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle,
    jstring watch_id,
    jint accuracy,
    jlong minimum_interval_ns,
    jdouble minimum_distance_meters,
    jboolean include_heading
) {
    if (!resolve_location_testing_symbols()) {
        return static_cast<jint>(HOST_STATUS_NOT_FOUND);
    }

    const char *watch_id_chars = env->GetStringUTFChars(watch_id, nullptr);
    jsize watch_id_length = env->GetStringUTFLength(watch_id);
    uint32_t status = test_location_watch_open(
        static_cast<uint64_t>(session_handle),
        NativeStringRef {
            .data = reinterpret_cast<const uint8_t *>(watch_id_chars),
            .len = static_cast<uint32_t>(watch_id_length),
        },
        LocationWatchOptions {
            .accuracy = static_cast<LocationAccuracy>(accuracy),
            .minimum_interval_ns = static_cast<uint64_t>(minimum_interval_ns),
            .minimum_distance_meters = minimum_distance_meters,
            .include_heading = include_heading == JNI_TRUE,
        }
    );

    env->ReleaseStringUTFChars(watch_id, watch_id_chars);

    return static_cast<jint>(status);
}

/// Close one location watch through the Android bridge test harness.
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeTestLocationWatchClose(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle,
    jstring watch_id
) {
    if (!resolve_location_testing_symbols()) {
        return static_cast<jint>(HOST_STATUS_NOT_FOUND);
    }

    const char *watch_id_chars = env->GetStringUTFChars(watch_id, nullptr);
    jsize watch_id_length = env->GetStringUTFLength(watch_id);
    uint32_t status = test_location_watch_close(
        static_cast<uint64_t>(session_handle),
        NativeStringRef {
            .data = reinterpret_cast<const uint8_t *>(watch_id_chars),
            .len = static_cast<uint32_t>(watch_id_length),
        }
    );

    env->ReleaseStringUTFChars(watch_id, watch_id_chars);

    return static_cast<jint>(status);
}
