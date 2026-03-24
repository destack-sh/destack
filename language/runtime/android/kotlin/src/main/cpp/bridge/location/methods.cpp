#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

namespace {

jclass bridge_class = nullptr;
jclass location_services_response_class = nullptr;
jclass location_last_known_response_class = nullptr;
jclass location_sample_class = nullptr;
jmethodID location_services_enabled_method = nullptr;
jmethodID location_last_known_method = nullptr;
jmethodID location_watch_open_method = nullptr;
jmethodID location_watch_close_method = nullptr;
jmethodID location_services_response_get_status_method = nullptr;
jmethodID location_services_response_get_is_enabled_method = nullptr;
jmethodID location_last_known_response_get_status_method = nullptr;
jmethodID location_last_known_response_get_sample_method = nullptr;
jmethodID location_sample_get_latitude_degrees_method = nullptr;
jmethodID location_sample_get_longitude_degrees_method = nullptr;
jmethodID location_sample_get_altitude_meters_method = nullptr;
jmethodID location_sample_get_horizontal_accuracy_meters_method = nullptr;
jmethodID location_sample_get_vertical_accuracy_meters_method = nullptr;
jmethodID location_sample_get_speed_meters_per_second_method = nullptr;
jmethodID location_sample_get_heading_degrees_method = nullptr;
jmethodID location_sample_get_timestamp_unix_ns_method = nullptr;

}

/// Decode one Java location sample into one native location sample payload.
static LocationSample decode_location_sample(JNIEnv *env, jobject value) {
    LocationSample sample = {};

    sample.latitude_degrees = env->CallDoubleMethod(
        value,
        location_sample_get_latitude_degrees_method
    );
    sample.longitude_degrees = env->CallDoubleMethod(
        value,
        location_sample_get_longitude_degrees_method
    );
    sample.altitude_meters = env->CallDoubleMethod(
        value,
        location_sample_get_altitude_meters_method
    );
    sample.horizontal_accuracy_meters = env->CallDoubleMethod(
        value,
        location_sample_get_horizontal_accuracy_meters_method
    );
    sample.vertical_accuracy_meters = env->CallDoubleMethod(
        value,
        location_sample_get_vertical_accuracy_meters_method
    );
    sample.speed_meters_per_second = env->CallDoubleMethod(
        value,
        location_sample_get_speed_meters_per_second_method
    );
    sample.heading_degrees = env->CallDoubleMethod(
        value,
        location_sample_get_heading_degrees_method
    );
    sample.timestamp_unix_ns = static_cast<uint64_t>(
        env->CallLongMethod(
            value,
            location_sample_get_timestamp_unix_ns_method
        )
    );

    return sample;
}

/// Resolve the location bridge methods from one runtime bridge instance.
bool resolve_location_methods(JNIEnv *env, jobject bridge) {
    if (bridge_class == nullptr) {
        jclass local_bridge_class = env->GetObjectClass(bridge);
        if (local_bridge_class == nullptr) {
            return false;
        }

        bridge_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_bridge_class));
        env->DeleteLocalRef(local_bridge_class);
        if (bridge_class == nullptr) {
            return false;
        }
    }

    if (location_services_enabled_method == nullptr) {
        location_services_enabled_method = env->GetMethodID(
            bridge_class,
            "locationServicesEnabled",
            "()Ldev/destack/runtime/android/module/location/RuntimeHostLocationServicesResponse;"
        );
    }

    if (location_last_known_method == nullptr) {
        location_last_known_method = env->GetMethodID(
            bridge_class,
            "locationLastKnown",
            "()Ldev/destack/runtime/android/module/location/RuntimeHostLocationLastKnownResponse;"
        );
    }

    if (location_watch_open_method == nullptr) {
        location_watch_open_method = env->GetMethodID(
            bridge_class,
            "locationWatchOpen",
            "(Ljava/lang/String;IJDZ)I"
        );
    }

    if (location_watch_close_method == nullptr) {
        location_watch_close_method = env->GetMethodID(
            bridge_class,
            "locationWatchClose",
            "(Ljava/lang/String;)I"
        );
    }

    if (location_services_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/location/RuntimeHostLocationServicesResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        location_services_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (location_services_response_class == nullptr) {
            return false;
        }
    }

    if (location_last_known_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/location/RuntimeHostLocationLastKnownResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        location_last_known_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (location_last_known_response_class == nullptr) {
            return false;
        }
    }

    if (location_sample_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/location/RuntimeHostLocationSample"
        );
        if (local_class == nullptr) {
            return false;
        }

        location_sample_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (location_sample_class == nullptr) {
            return false;
        }
    }

    if (location_services_response_get_status_method == nullptr) {
        location_services_response_get_status_method = env->GetMethodID(
            location_services_response_class,
            "getStatus",
            "()I"
        );
    }

    if (location_services_response_get_is_enabled_method == nullptr) {
        location_services_response_get_is_enabled_method = env->GetMethodID(
            location_services_response_class,
            "isEnabled",
            "()Z"
        );
    }

    if (location_last_known_response_get_status_method == nullptr) {
        location_last_known_response_get_status_method = env->GetMethodID(
            location_last_known_response_class,
            "getStatus",
            "()I"
        );
    }

    if (location_last_known_response_get_sample_method == nullptr) {
        location_last_known_response_get_sample_method = env->GetMethodID(
            location_last_known_response_class,
            "getSample",
            "()Ldev/destack/runtime/android/module/location/RuntimeHostLocationSample;"
        );
    }

    if (location_sample_get_latitude_degrees_method == nullptr) {
        location_sample_get_latitude_degrees_method = env->GetMethodID(
            location_sample_class,
            "getLatitudeDegrees",
            "()D"
        );
    }

    if (location_sample_get_longitude_degrees_method == nullptr) {
        location_sample_get_longitude_degrees_method = env->GetMethodID(
            location_sample_class,
            "getLongitudeDegrees",
            "()D"
        );
    }

    if (location_sample_get_altitude_meters_method == nullptr) {
        location_sample_get_altitude_meters_method = env->GetMethodID(
            location_sample_class,
            "getAltitudeMeters",
            "()D"
        );
    }

    if (location_sample_get_horizontal_accuracy_meters_method == nullptr) {
        location_sample_get_horizontal_accuracy_meters_method = env->GetMethodID(
            location_sample_class,
            "getHorizontalAccuracyMeters",
            "()D"
        );
    }

    if (location_sample_get_vertical_accuracy_meters_method == nullptr) {
        location_sample_get_vertical_accuracy_meters_method = env->GetMethodID(
            location_sample_class,
            "getVerticalAccuracyMeters",
            "()D"
        );
    }

    if (location_sample_get_speed_meters_per_second_method == nullptr) {
        location_sample_get_speed_meters_per_second_method = env->GetMethodID(
            location_sample_class,
            "getSpeedMetersPerSecond",
            "()D"
        );
    }

    if (location_sample_get_heading_degrees_method == nullptr) {
        location_sample_get_heading_degrees_method = env->GetMethodID(
            location_sample_class,
            "getHeadingDegrees",
            "()D"
        );
    }

    if (location_sample_get_timestamp_unix_ns_method == nullptr) {
        location_sample_get_timestamp_unix_ns_method = env->GetMethodID(
            location_sample_class,
            "getTimestampUnixNs",
            "()J"
        );
    }

    return
        location_services_enabled_method != nullptr &&
        location_last_known_method != nullptr &&
        location_watch_open_method != nullptr &&
        location_watch_close_method != nullptr &&
        location_services_response_get_status_method != nullptr &&
        location_services_response_get_is_enabled_method != nullptr &&
        location_last_known_response_get_status_method != nullptr &&
        location_last_known_response_get_sample_method != nullptr &&
        location_sample_get_latitude_degrees_method != nullptr &&
        location_sample_get_longitude_degrees_method != nullptr &&
        location_sample_get_altitude_meters_method != nullptr &&
        location_sample_get_horizontal_accuracy_meters_method != nullptr &&
        location_sample_get_vertical_accuracy_meters_method != nullptr &&
        location_sample_get_speed_meters_per_second_method != nullptr &&
        location_sample_get_heading_degrees_method != nullptr &&
        location_sample_get_timestamp_unix_ns_method != nullptr;
}

/// Call the location-services-enabled entrypoint on one registered bridge.
uint32_t call_location_services_enabled(
    JNIEnv *env,
    uint64_t session_handle,
    bool *is_enabled
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        location_services_enabled_method
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck() || response == nullptr) {
        env->ExceptionClear();
        if (response != nullptr) {
            env->DeleteLocalRef(response);
        }
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        response,
        location_services_response_get_status_method
    );
    jboolean enabled = env->CallBooleanMethod(
        response,
        location_services_response_get_is_enabled_method
    );
    env->DeleteLocalRef(response);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    *is_enabled = enabled == JNI_TRUE;
    return static_cast<uint32_t>(status);
}

/// Call the last-known-location entrypoint on one registered bridge.
uint32_t call_location_last_known(
    JNIEnv *env,
    uint64_t session_handle,
    LocationSample *sample
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        location_last_known_method
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck() || response == nullptr) {
        env->ExceptionClear();
        if (response != nullptr) {
            env->DeleteLocalRef(response);
        }
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        response,
        location_last_known_response_get_status_method
    );
    jobject response_sample = env->CallObjectMethod(
        response,
        location_last_known_response_get_sample_method
    );
    env->DeleteLocalRef(response);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        if (response_sample != nullptr) {
            env->DeleteLocalRef(response_sample);
        }
        return HOST_STATUS_FAILED;
    }

    if (response_sample != nullptr) {
        *sample = decode_location_sample(env, response_sample);
        env->DeleteLocalRef(response_sample);
    }

    return static_cast<uint32_t>(status);
}

/// Call the location-watch-open entrypoint on one registered bridge.
uint32_t call_location_watch_open(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef watch_id,
    LocationWatchOptions options
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring watch_id_string = new_java_string(env, watch_id);
    if (watch_id_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        location_watch_open_method,
        watch_id_string,
        static_cast<jint>(options.accuracy),
        static_cast<jlong>(options.minimum_interval_ns),
        static_cast<jdouble>(options.minimum_distance_meters),
        static_cast<jboolean>(options.include_heading)
    );
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(watch_id_string);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the location-watch-close entrypoint on one registered bridge.
uint32_t call_location_watch_close(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef watch_id
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring watch_id_string = new_java_string(env, watch_id);
    if (watch_id_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        location_watch_close_method,
        watch_id_string
    );
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(watch_id_string);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
