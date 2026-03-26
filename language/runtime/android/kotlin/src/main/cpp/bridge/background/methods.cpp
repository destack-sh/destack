#include "../jni.h"
#include "../registry.h"
#include "../types.h"
#include "methods.h"

#include <deque>
#include <string>
#include <vector>

namespace {

jclass bridge_class = nullptr;
jclass list_class = nullptr;
jclass enum_class = nullptr;
jclass status_response_class = nullptr;
jclass task_list_response_class = nullptr;
jclass trigger_response_class = nullptr;
jclass descriptor_class = nullptr;
jmethodID background_status_method = nullptr;
jmethodID list_background_tasks_method = nullptr;
jmethodID register_background_task_method = nullptr;
jmethodID unregister_background_task_method = nullptr;
jmethodID trigger_background_task_method = nullptr;
jmethodID complete_background_task_method = nullptr;
jmethodID list_size_method = nullptr;
jmethodID list_get_method = nullptr;
jmethodID enum_ordinal_method = nullptr;
jmethodID status_response_get_status_method = nullptr;
jmethodID status_response_get_scheduler_status_method = nullptr;
jmethodID task_list_response_get_status_method = nullptr;
jmethodID task_list_response_get_descriptors_method = nullptr;
jmethodID trigger_response_get_status_method = nullptr;
jmethodID trigger_response_get_is_triggered_method = nullptr;
jmethodID descriptor_get_identifier_method = nullptr;
jmethodID descriptor_get_trigger_method = nullptr;
jmethodID descriptor_get_schedule_kind_method = nullptr;
jmethodID descriptor_get_has_earliest_begin_unix_ns_method = nullptr;
jmethodID descriptor_get_earliest_begin_unix_ns_method = nullptr;
jmethodID descriptor_get_has_repeat_interval_ns_method = nullptr;
jmethodID descriptor_get_repeat_interval_ns_method = nullptr;
jmethodID descriptor_get_network_method = nullptr;
jmethodID descriptor_get_requires_charging_method = nullptr;
jmethodID descriptor_get_requires_idle_method = nullptr;
jmethodID descriptor_get_conflict_policy_method = nullptr;
thread_local std::deque<std::string> result_string_storage;
thread_local std::vector<HostBackgroundTaskDescriptor> result_descriptor_storage;

/// Convert one Java string into one native string reference backed by stable storage.
NativeStringRef string_ref_from_java(
    JNIEnv *env,
    jstring value,
    std::deque<std::string> *storage
) {
    if (value == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    if (chars == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    jsize length = env->GetStringUTFLength(value);
    storage->emplace_back(chars, chars + length);
    env->ReleaseStringUTFChars(value, chars);

    const std::string &owned = storage->back();

    return NativeStringRef {
        .data = reinterpret_cast<const uint8_t *>(owned.data()),
        .len = static_cast<uint32_t>(owned.size()),
    };
}

/// Resolve the shared java.util.List helpers.
bool resolve_list_methods(JNIEnv *env) {
    if (list_class == nullptr) {
        jclass local_class = env->FindClass("java/util/List");
        if (local_class == nullptr) {
            return false;
        }

        list_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (list_class == nullptr) {
            return false;
        }
    }

    if (list_size_method == nullptr) {
        list_size_method = env->GetMethodID(list_class, "size", "()I");
    }

    if (list_get_method == nullptr) {
        list_get_method = env->GetMethodID(list_class, "get", "(I)Ljava/lang/Object;");
    }

    return list_size_method != nullptr && list_get_method != nullptr;
}

/// Resolve the shared java.lang.Enum helpers.
bool resolve_enum_methods(JNIEnv *env) {
    if (enum_class == nullptr) {
        jclass local_class = env->FindClass("java/lang/Enum");
        if (local_class == nullptr) {
            return false;
        }

        enum_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (enum_class == nullptr) {
            return false;
        }
    }

    if (enum_ordinal_method == nullptr) {
        enum_ordinal_method = env->GetMethodID(enum_class, "ordinal", "()I");
    }

    return enum_ordinal_method != nullptr;
}

/// Resolve the background bridge entrypoints from one runtime bridge instance.
bool resolve_bridge_methods(JNIEnv *env, jobject bridge) {
    if (bridge_class == nullptr) {
        jclass local_class = env->GetObjectClass(bridge);
        if (local_class == nullptr) {
            return false;
        }

        bridge_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (bridge_class == nullptr) {
            return false;
        }
    }

    if (background_status_method == nullptr) {
        background_status_method = env->GetMethodID(
            bridge_class,
            "backgroundStatus",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundStatusResponse;"
        );
    }

    if (list_background_tasks_method == nullptr) {
        list_background_tasks_method = env->GetMethodID(
            bridge_class,
            "listBackgroundTasks",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundTaskListResponse;"
        );
    }

    if (register_background_task_method == nullptr) {
        register_background_task_method = env->GetMethodID(
            bridge_class,
            "registerBackgroundTask",
            "(Ljava/lang/String;IIZJZJIZZI)I"
        );
    }

    if (unregister_background_task_method == nullptr) {
        unregister_background_task_method = env->GetMethodID(
            bridge_class,
            "unregisterBackgroundTask",
            "(Ljava/lang/String;)I"
        );
    }

    if (trigger_background_task_method == nullptr) {
        trigger_background_task_method = env->GetMethodID(
            bridge_class,
            "triggerBackgroundTask",
            "(Ljava/lang/String;)Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundTriggerResponse;"
        );
    }

    if (complete_background_task_method == nullptr) {
        complete_background_task_method = env->GetMethodID(
            bridge_class,
            "completeBackgroundTask",
            "(Ljava/lang/String;I)I"
        );
    }

    return
        background_status_method != nullptr &&
        list_background_tasks_method != nullptr &&
        register_background_task_method != nullptr &&
        unregister_background_task_method != nullptr &&
        trigger_background_task_method != nullptr &&
        complete_background_task_method != nullptr;
}

/// Resolve the shared background response and payload accessors.
bool resolve_background_types(JNIEnv *env) {
    if (status_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/background/RuntimeHostBackgroundStatusResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        status_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (status_response_class == nullptr) {
            return false;
        }
    }

    if (task_list_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/background/RuntimeHostBackgroundTaskListResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        task_list_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (task_list_response_class == nullptr) {
            return false;
        }
    }

    if (trigger_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/background/RuntimeHostBackgroundTriggerResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        trigger_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (trigger_response_class == nullptr) {
            return false;
        }
    }

    if (descriptor_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/background/RuntimeHostBackgroundTaskDescriptor"
        );
        if (local_class == nullptr) {
            return false;
        }

        descriptor_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (descriptor_class == nullptr) {
            return false;
        }
    }

    if (status_response_get_status_method == nullptr) {
        status_response_get_status_method =
            env->GetMethodID(status_response_class, "getStatus", "()I");
    }

    if (status_response_get_scheduler_status_method == nullptr) {
        status_response_get_scheduler_status_method = env->GetMethodID(
            status_response_class,
            "getSchedulerStatus",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundStatus;"
        );
    }

    if (task_list_response_get_status_method == nullptr) {
        task_list_response_get_status_method =
            env->GetMethodID(task_list_response_class, "getStatus", "()I");
    }

    if (task_list_response_get_descriptors_method == nullptr) {
        task_list_response_get_descriptors_method =
            env->GetMethodID(task_list_response_class, "getDescriptors", "()Ljava/util/List;");
    }

    if (trigger_response_get_status_method == nullptr) {
        trigger_response_get_status_method =
            env->GetMethodID(trigger_response_class, "getStatus", "()I");
    }

    if (trigger_response_get_is_triggered_method == nullptr) {
        trigger_response_get_is_triggered_method =
            env->GetMethodID(trigger_response_class, "isTriggered", "()Z");
    }

    if (descriptor_get_identifier_method == nullptr) {
        descriptor_get_identifier_method =
            env->GetMethodID(descriptor_class, "getIdentifier", "()Ljava/lang/String;");
    }

    if (descriptor_get_trigger_method == nullptr) {
        descriptor_get_trigger_method = env->GetMethodID(
            descriptor_class,
            "getTrigger",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundTriggerKind;"
        );
    }

    if (descriptor_get_schedule_kind_method == nullptr) {
        descriptor_get_schedule_kind_method = env->GetMethodID(
            descriptor_class,
            "getScheduleKind",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundTaskScheduleKind;"
        );
    }

    if (descriptor_get_has_earliest_begin_unix_ns_method == nullptr) {
        descriptor_get_has_earliest_begin_unix_ns_method =
            env->GetMethodID(descriptor_class, "getHasEarliestBeginUnixNs", "()Z");
    }

    if (descriptor_get_earliest_begin_unix_ns_method == nullptr) {
        descriptor_get_earliest_begin_unix_ns_method =
            env->GetMethodID(descriptor_class, "getEarliestBeginUnixNs", "()J");
    }

    if (descriptor_get_has_repeat_interval_ns_method == nullptr) {
        descriptor_get_has_repeat_interval_ns_method =
            env->GetMethodID(descriptor_class, "getHasRepeatIntervalNs", "()Z");
    }

    if (descriptor_get_repeat_interval_ns_method == nullptr) {
        descriptor_get_repeat_interval_ns_method =
            env->GetMethodID(descriptor_class, "getRepeatIntervalNs", "()J");
    }

    if (descriptor_get_network_method == nullptr) {
        descriptor_get_network_method = env->GetMethodID(
            descriptor_class,
            "getNetwork",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundNetworkRequirement;"
        );
    }

    if (descriptor_get_requires_charging_method == nullptr) {
        descriptor_get_requires_charging_method =
            env->GetMethodID(descriptor_class, "getRequiresCharging", "()Z");
    }

    if (descriptor_get_requires_idle_method == nullptr) {
        descriptor_get_requires_idle_method =
            env->GetMethodID(descriptor_class, "getRequiresIdle", "()Z");
    }

    if (descriptor_get_conflict_policy_method == nullptr) {
        descriptor_get_conflict_policy_method = env->GetMethodID(
            descriptor_class,
            "getConflictPolicy",
            "()Ldev/destack/runtime/android/module/background/RuntimeHostBackgroundConflictPolicy;"
        );
    }

    return
        status_response_get_status_method != nullptr &&
        status_response_get_scheduler_status_method != nullptr &&
        task_list_response_get_status_method != nullptr &&
        task_list_response_get_descriptors_method != nullptr &&
        trigger_response_get_status_method != nullptr &&
        trigger_response_get_is_triggered_method != nullptr &&
        descriptor_get_identifier_method != nullptr &&
        descriptor_get_trigger_method != nullptr &&
        descriptor_get_schedule_kind_method != nullptr &&
        descriptor_get_has_earliest_begin_unix_ns_method != nullptr &&
        descriptor_get_earliest_begin_unix_ns_method != nullptr &&
        descriptor_get_has_repeat_interval_ns_method != nullptr &&
        descriptor_get_repeat_interval_ns_method != nullptr &&
        descriptor_get_network_method != nullptr &&
        descriptor_get_requires_charging_method != nullptr &&
        descriptor_get_requires_idle_method != nullptr &&
        descriptor_get_conflict_policy_method != nullptr;
}

/// Decode one Java background status enum into one bridge value.
HostBackgroundStatus decode_status(JNIEnv *env, jobject value) {
    jint ordinal = env->CallIntMethod(value, enum_ordinal_method);

    return static_cast<HostBackgroundStatus>(ordinal + 1);
}

/// Decode one Java background trigger enum into one bridge value.
HostBackgroundTriggerKind decode_trigger(JNIEnv *env, jobject value) {
    jint ordinal = env->CallIntMethod(value, enum_ordinal_method);

    return static_cast<HostBackgroundTriggerKind>(ordinal + 1);
}

/// Decode one Java background schedule-kind enum into one bridge value.
HostBackgroundTaskScheduleKind decode_schedule_kind(JNIEnv *env, jobject value) {
    jint ordinal = env->CallIntMethod(value, enum_ordinal_method);

    return static_cast<HostBackgroundTaskScheduleKind>(ordinal + 1);
}

/// Decode one Java background network requirement enum into one bridge value.
HostBackgroundNetworkRequirement decode_network(JNIEnv *env, jobject value) {
    jint ordinal = env->CallIntMethod(value, enum_ordinal_method);

    return static_cast<HostBackgroundNetworkRequirement>(ordinal + 1);
}

/// Decode one Java background conflict policy enum into one bridge value.
HostBackgroundConflictPolicy decode_conflict_policy(JNIEnv *env, jobject value) {
    jint ordinal = env->CallIntMethod(value, enum_ordinal_method);

    return static_cast<HostBackgroundConflictPolicy>(ordinal + 1);
}

/// Decode one Java background descriptor into one bridge value.
HostBackgroundTaskDescriptor decode_descriptor(JNIEnv *env, jobject value) {
    jstring identifier = static_cast<jstring>(
        env->CallObjectMethod(value, descriptor_get_identifier_method)
    );
    jobject trigger = env->CallObjectMethod(value, descriptor_get_trigger_method);
    jobject schedule_kind = env->CallObjectMethod(value, descriptor_get_schedule_kind_method);
    jobject network = env->CallObjectMethod(value, descriptor_get_network_method);
    jobject conflict_policy =
        env->CallObjectMethod(value, descriptor_get_conflict_policy_method);

    HostBackgroundTaskDescriptor descriptor = {
        .identifier = string_ref_from_java(env, identifier, &result_string_storage),
        .trigger = decode_trigger(env, trigger),
        .schedule = {
            .kind = decode_schedule_kind(env, schedule_kind),
            .has_earliest_begin_unix_ns =
                env->CallBooleanMethod(
                    value,
                    descriptor_get_has_earliest_begin_unix_ns_method
                ) == JNI_TRUE,
            .earliest_begin_unix_ns = static_cast<uint64_t>(
                env->CallLongMethod(value, descriptor_get_earliest_begin_unix_ns_method)
            ),
            .has_repeat_interval_ns =
                env->CallBooleanMethod(
                    value,
                    descriptor_get_has_repeat_interval_ns_method
                ) == JNI_TRUE,
            .repeat_interval_ns = static_cast<uint64_t>(
                env->CallLongMethod(value, descriptor_get_repeat_interval_ns_method)
            ),
        },
        .network = decode_network(env, network),
        .requires_charging =
            env->CallBooleanMethod(value, descriptor_get_requires_charging_method) == JNI_TRUE,
        .requires_idle = env->CallBooleanMethod(value, descriptor_get_requires_idle_method) == JNI_TRUE,
        .conflict_policy = decode_conflict_policy(env, conflict_policy),
    };

    env->DeleteLocalRef(identifier);
    env->DeleteLocalRef(trigger);
    env->DeleteLocalRef(schedule_kind);
    env->DeleteLocalRef(network);
    env->DeleteLocalRef(conflict_policy);

    return descriptor;
}

} // namespace

/// Resolve the background bridge methods from one runtime bridge instance.
bool resolve_background_methods(JNIEnv *env, jobject bridge) {
    return
        resolve_list_methods(env) &&
        resolve_enum_methods(env) &&
        resolve_bridge_methods(env, bridge) &&
        resolve_background_types(env);
}

/// Call the background-status entrypoint on one registered bridge.
uint32_t call_background_status(
    JNIEnv *env,
    uint64_t session_handle,
    HostBackgroundStatus *output_status
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobject response = env->CallObjectMethod(bridge, background_status_method);
    if (response == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, status_response_get_status_method);
    if (status == HOST_STATUS_OK) {
        jobject scheduler_status =
            env->CallObjectMethod(response, status_response_get_scheduler_status_method);
        if (scheduler_status == nullptr) {
            status = HOST_STATUS_FAILED;
        } else {
            *output_status = decode_status(env, scheduler_status);
            env->DeleteLocalRef(scheduler_status);
        }
    }

    env->DeleteLocalRef(response);
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the background-list entrypoint on one registered bridge.
uint32_t call_background_list(
    JNIEnv *env,
    uint64_t session_handle,
    NativeArray<HostBackgroundTaskDescriptor> *output_descriptors
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobject response = env->CallObjectMethod(bridge, list_background_tasks_method);
    if (response == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, task_list_response_get_status_method);
    if (status == HOST_STATUS_OK) {
        jobject descriptors =
            env->CallObjectMethod(response, task_list_response_get_descriptors_method);
        if (descriptors == nullptr) {
            status = HOST_STATUS_FAILED;
        } else {
            result_string_storage.clear();
            result_descriptor_storage.clear();

            jint count = env->CallIntMethod(descriptors, list_size_method);
            result_descriptor_storage.reserve(static_cast<size_t>(count));

            for (jint index = 0; index < count; index += 1) {
                jobject descriptor =
                    env->CallObjectMethod(descriptors, list_get_method, index);
                if (descriptor == nullptr) {
                    status = HOST_STATUS_FAILED;
                    break;
                }

                result_descriptor_storage.push_back(decode_descriptor(env, descriptor));
                env->DeleteLocalRef(descriptor);
            }

            if (status == HOST_STATUS_OK) {
                output_descriptors->data =
                    result_descriptor_storage.empty() ? nullptr : result_descriptor_storage.data();
                output_descriptors->len =
                    static_cast<uint32_t>(result_descriptor_storage.size());
                output_descriptors->capacity =
                    static_cast<uint32_t>(result_descriptor_storage.capacity());
            }

            env->DeleteLocalRef(descriptors);
        }
    }

    env->DeleteLocalRef(response);
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the background-register entrypoint on one registered bridge.
uint32_t call_background_register(
    JNIEnv *env,
    uint64_t session_handle,
    HostBackgroundTaskOptions options
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier = new_java_string(env, options.identifier);
    jint status = env->CallIntMethod(
        bridge,
        register_background_task_method,
        identifier,
        static_cast<jint>(options.trigger),
        static_cast<jint>(options.schedule.kind),
        options.schedule.has_earliest_begin_unix_ns ? JNI_TRUE : JNI_FALSE,
        static_cast<jlong>(options.schedule.earliest_begin_unix_ns),
        options.schedule.has_repeat_interval_ns ? JNI_TRUE : JNI_FALSE,
        static_cast<jlong>(options.schedule.repeat_interval_ns),
        static_cast<jint>(options.network),
        options.requires_charging ? JNI_TRUE : JNI_FALSE,
        options.requires_idle ? JNI_TRUE : JNI_FALSE,
        static_cast<jint>(options.conflict_policy)
    );

    if (identifier != nullptr) {
        env->DeleteLocalRef(identifier);
    }
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the background-unregister entrypoint on one registered bridge.
uint32_t call_background_unregister(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef identifier
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring java_identifier = new_java_string(env, identifier);
    jint status = env->CallIntMethod(
        bridge,
        unregister_background_task_method,
        java_identifier
    );

    if (java_identifier != nullptr) {
        env->DeleteLocalRef(java_identifier);
    }
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the background-trigger entrypoint on one registered bridge.
uint32_t call_background_trigger_test(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef identifier,
    bool *is_triggered
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring java_identifier = new_java_string(env, identifier);
    jobject response = env->CallObjectMethod(
        bridge,
        trigger_background_task_method,
        java_identifier
    );
    if (java_identifier != nullptr) {
        env->DeleteLocalRef(java_identifier);
    }
    if (response == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, trigger_response_get_status_method);
    if (status == HOST_STATUS_OK) {
        *is_triggered =
            env->CallBooleanMethod(response, trigger_response_get_is_triggered_method) ==
            JNI_TRUE;
    }

    env->DeleteLocalRef(response);
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the background-complete entrypoint on one registered bridge.
uint32_t call_background_complete(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef execution_id,
    HostBackgroundTaskResult result
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring java_execution_id = new_java_string(env, execution_id);
    jint status = env->CallIntMethod(
        bridge,
        complete_background_task_method,
        java_execution_id,
        static_cast<jint>(result)
    );

    if (java_execution_id != nullptr) {
        env->DeleteLocalRef(java_execution_id);
    }
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
