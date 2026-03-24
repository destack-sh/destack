#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

#include <deque>
#include <string>
#include <vector>

namespace {

jclass bridge_class = nullptr;
jclass list_class = nullptr;
jclass enum_class = nullptr;
jclass media_list_response_class = nullptr;
jclass media_list_result_class = nullptr;
jclass media_read_response_class = nullptr;
jclass media_import_response_class = nullptr;
jclass media_delete_response_class = nullptr;
jclass media_asset_descriptor_class = nullptr;
jmethodID list_media_method = nullptr;
jmethodID read_media_method = nullptr;
jmethodID import_media_path_method = nullptr;
jmethodID delete_media_method = nullptr;
jmethodID list_size_method = nullptr;
jmethodID list_get_method = nullptr;
jmethodID enum_ordinal_method = nullptr;
jmethodID media_list_response_get_status_method = nullptr;
jmethodID media_list_response_get_page_method = nullptr;
jmethodID media_list_result_get_assets_method = nullptr;
jmethodID media_list_result_get_next_cursor_method = nullptr;
jmethodID media_list_result_get_has_more_method = nullptr;
jmethodID media_read_response_get_status_method = nullptr;
jmethodID media_read_response_get_descriptor_method = nullptr;
jmethodID media_import_response_get_status_method = nullptr;
jmethodID media_import_response_get_identifier_method = nullptr;
jmethodID media_delete_response_get_status_method = nullptr;
jmethodID media_delete_response_get_deleted_count_method = nullptr;
jmethodID media_asset_descriptor_get_identifier_method = nullptr;
jmethodID media_asset_descriptor_get_uri_method = nullptr;
jmethodID media_asset_descriptor_get_filename_method = nullptr;
jmethodID media_asset_descriptor_get_mime_type_method = nullptr;
jmethodID media_asset_descriptor_get_kind_method = nullptr;
jmethodID media_asset_descriptor_get_width_method = nullptr;
jmethodID media_asset_descriptor_get_height_method = nullptr;
jmethodID media_asset_descriptor_get_duration_ms_method = nullptr;
jmethodID media_asset_descriptor_get_size_bytes_method = nullptr;
jmethodID media_asset_descriptor_get_created_unix_ns_method = nullptr;
jmethodID media_asset_descriptor_get_modified_unix_ns_method = nullptr;
thread_local std::deque<std::string> result_string_storage;
thread_local std::vector<HostMediaAssetDescriptor> result_descriptor_storage;

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

/// Resolve the bridge entrypoint methods from one runtime bridge instance.
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

    if (list_media_method == nullptr) {
        list_media_method = env->GetMethodID(
            bridge_class,
            "listMedia",
            "(Ljava/lang/String;ZI[IZ)Ldev/destack/runtime/android/module/media/RuntimeHostMediaListResponse;"
        );
    }

    if (read_media_method == nullptr) {
        read_media_method = env->GetMethodID(
            bridge_class,
            "readMedia",
            "(Ljava/lang/String;)Ldev/destack/runtime/android/module/media/RuntimeHostMediaReadResponse;"
        );
    }

    if (import_media_path_method == nullptr) {
        import_media_path_method = env->GetMethodID(
            bridge_class,
            "importMediaPath",
            "(Ljava/lang/String;I)Ldev/destack/runtime/android/module/media/RuntimeHostMediaImportPathResponse;"
        );
    }

    if (delete_media_method == nullptr) {
        delete_media_method = env->GetMethodID(
            bridge_class,
            "deleteMedia",
            "([Ljava/lang/String;)Ldev/destack/runtime/android/module/media/RuntimeHostMediaDeleteResponse;"
        );
    }

    return
        list_media_method != nullptr &&
        read_media_method != nullptr &&
        import_media_path_method != nullptr &&
        delete_media_method != nullptr;
}

/// Resolve the response and payload accessors for one media callback path.
bool resolve_response_methods(JNIEnv *env) {
    if (media_list_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/media/RuntimeHostMediaListResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        media_list_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (media_list_response_class == nullptr) {
            return false;
        }
    }

    if (media_list_result_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/media/RuntimeHostMediaListResult"
        );
        if (local_class == nullptr) {
            return false;
        }

        media_list_result_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (media_list_result_class == nullptr) {
            return false;
        }
    }

    if (media_read_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/media/RuntimeHostMediaReadResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        media_read_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (media_read_response_class == nullptr) {
            return false;
        }
    }

    if (media_import_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/media/RuntimeHostMediaImportPathResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        media_import_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (media_import_response_class == nullptr) {
            return false;
        }
    }

    if (media_delete_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/media/RuntimeHostMediaDeleteResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        media_delete_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (media_delete_response_class == nullptr) {
            return false;
        }
    }

    if (media_asset_descriptor_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/media/RuntimeHostMediaAssetDescriptor"
        );
        if (local_class == nullptr) {
            return false;
        }

        media_asset_descriptor_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (media_asset_descriptor_class == nullptr) {
            return false;
        }
    }

    if (media_list_response_get_status_method == nullptr) {
        media_list_response_get_status_method = env->GetMethodID(
            media_list_response_class,
            "getStatus",
            "()I"
        );
    }

    if (media_list_response_get_page_method == nullptr) {
        media_list_response_get_page_method = env->GetMethodID(
            media_list_response_class,
            "getPage",
            "()Ldev/destack/runtime/android/module/media/RuntimeHostMediaListResult;"
        );
    }

    if (media_list_result_get_assets_method == nullptr) {
        media_list_result_get_assets_method = env->GetMethodID(
            media_list_result_class,
            "getAssets",
            "()Ljava/util/List;"
        );
    }

    if (media_list_result_get_next_cursor_method == nullptr) {
        media_list_result_get_next_cursor_method = env->GetMethodID(
            media_list_result_class,
            "getNextCursor",
            "()Ljava/lang/String;"
        );
    }

    if (media_list_result_get_has_more_method == nullptr) {
        media_list_result_get_has_more_method = env->GetMethodID(
            media_list_result_class,
            "getHasMore",
            "()Z"
        );
    }

    if (media_read_response_get_status_method == nullptr) {
        media_read_response_get_status_method = env->GetMethodID(
            media_read_response_class,
            "getStatus",
            "()I"
        );
    }

    if (media_read_response_get_descriptor_method == nullptr) {
        media_read_response_get_descriptor_method = env->GetMethodID(
            media_read_response_class,
            "getDescriptor",
            "()Ldev/destack/runtime/android/module/media/RuntimeHostMediaAssetDescriptor;"
        );
    }

    if (media_import_response_get_status_method == nullptr) {
        media_import_response_get_status_method = env->GetMethodID(
            media_import_response_class,
            "getStatus",
            "()I"
        );
    }

    if (media_import_response_get_identifier_method == nullptr) {
        media_import_response_get_identifier_method = env->GetMethodID(
            media_import_response_class,
            "getIdentifier",
            "()Ljava/lang/String;"
        );
    }

    if (media_delete_response_get_status_method == nullptr) {
        media_delete_response_get_status_method = env->GetMethodID(
            media_delete_response_class,
            "getStatus",
            "()I"
        );
    }

    if (media_delete_response_get_deleted_count_method == nullptr) {
        media_delete_response_get_deleted_count_method = env->GetMethodID(
            media_delete_response_class,
            "getDeletedCount",
            "()I"
        );
    }

    if (media_asset_descriptor_get_identifier_method == nullptr) {
        media_asset_descriptor_get_identifier_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getIdentifier",
            "()Ljava/lang/String;"
        );
    }

    if (media_asset_descriptor_get_uri_method == nullptr) {
        media_asset_descriptor_get_uri_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getUri",
            "()Ljava/lang/String;"
        );
    }

    if (media_asset_descriptor_get_filename_method == nullptr) {
        media_asset_descriptor_get_filename_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getFilename",
            "()Ljava/lang/String;"
        );
    }

    if (media_asset_descriptor_get_mime_type_method == nullptr) {
        media_asset_descriptor_get_mime_type_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getMimeType",
            "()Ljava/lang/String;"
        );
    }

    if (media_asset_descriptor_get_kind_method == nullptr) {
        media_asset_descriptor_get_kind_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getKind",
            "()Ldev/destack/runtime/android/module/media/RuntimeHostMediaAssetKind;"
        );
    }

    if (media_asset_descriptor_get_width_method == nullptr) {
        media_asset_descriptor_get_width_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getWidth",
            "()I"
        );
    }

    if (media_asset_descriptor_get_height_method == nullptr) {
        media_asset_descriptor_get_height_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getHeight",
            "()I"
        );
    }

    if (media_asset_descriptor_get_duration_ms_method == nullptr) {
        media_asset_descriptor_get_duration_ms_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getDurationMs",
            "()J"
        );
    }

    if (media_asset_descriptor_get_size_bytes_method == nullptr) {
        media_asset_descriptor_get_size_bytes_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getSizeBytes",
            "()J"
        );
    }

    if (media_asset_descriptor_get_created_unix_ns_method == nullptr) {
        media_asset_descriptor_get_created_unix_ns_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getCreatedUnixNs",
            "()J"
        );
    }

    if (media_asset_descriptor_get_modified_unix_ns_method == nullptr) {
        media_asset_descriptor_get_modified_unix_ns_method = env->GetMethodID(
            media_asset_descriptor_class,
            "getModifiedUnixNs",
            "()J"
        );
    }

    return
        media_list_response_get_status_method != nullptr &&
        media_list_response_get_page_method != nullptr &&
        media_list_result_get_assets_method != nullptr &&
        media_list_result_get_next_cursor_method != nullptr &&
        media_list_result_get_has_more_method != nullptr &&
        media_read_response_get_status_method != nullptr &&
        media_read_response_get_descriptor_method != nullptr &&
        media_import_response_get_status_method != nullptr &&
        media_import_response_get_identifier_method != nullptr &&
        media_delete_response_get_status_method != nullptr &&
        media_delete_response_get_deleted_count_method != nullptr &&
        media_asset_descriptor_get_identifier_method != nullptr &&
        media_asset_descriptor_get_uri_method != nullptr &&
        media_asset_descriptor_get_filename_method != nullptr &&
        media_asset_descriptor_get_mime_type_method != nullptr &&
        media_asset_descriptor_get_kind_method != nullptr &&
        media_asset_descriptor_get_width_method != nullptr &&
        media_asset_descriptor_get_height_method != nullptr &&
        media_asset_descriptor_get_duration_ms_method != nullptr &&
        media_asset_descriptor_get_size_bytes_method != nullptr &&
        media_asset_descriptor_get_created_unix_ns_method != nullptr &&
        media_asset_descriptor_get_modified_unix_ns_method != nullptr &&
        resolve_list_methods(env) &&
        resolve_enum_methods(env);
}

/// Convert one bridge media-kind slice into one Java int array.
jintArray new_media_kind_array(JNIEnv *env, HostMediaAssetKindSlice kinds) {
    jintArray array = env->NewIntArray(static_cast<jsize>(kinds.len));
    if (array == nullptr) {
        return nullptr;
    }

    if (kinds.len == 0) {
        return array;
    }

    std::vector<jint> values;
    values.reserve(static_cast<size_t>(kinds.len));

    for (uint32_t index = 0; index < kinds.len; ++index) {
        values.push_back(static_cast<jint>(kinds.data[index]));
    }

    env->SetIntArrayRegion(
        array,
        0,
        static_cast<jsize>(values.size()),
        values.data()
    );

    return array;
}

/// Convert one Java enum into one bridge media kind.
HostMediaAssetKind media_kind_from_enum(JNIEnv *env, jobject kind) {
    jint ordinal = env->CallIntMethod(kind, enum_ordinal_method);

    return static_cast<HostMediaAssetKind>(ordinal + 1);
}

/// Decode one Java media descriptor into one native descriptor.
bool decode_media_descriptor(
    JNIEnv *env,
    jobject descriptor,
    HostMediaAssetDescriptor *out_descriptor,
    std::deque<std::string> *string_storage
) {
    jstring identifier = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, media_asset_descriptor_get_identifier_method)
    );
    jstring uri = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, media_asset_descriptor_get_uri_method)
    );
    jstring filename = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, media_asset_descriptor_get_filename_method)
    );
    jstring mime_type = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, media_asset_descriptor_get_mime_type_method)
    );
    jobject kind = env->CallObjectMethod(descriptor, media_asset_descriptor_get_kind_method);

    if (identifier == nullptr || uri == nullptr || filename == nullptr || mime_type == nullptr || kind == nullptr) {
        if (identifier != nullptr) env->DeleteLocalRef(identifier);
        if (uri != nullptr) env->DeleteLocalRef(uri);
        if (filename != nullptr) env->DeleteLocalRef(filename);
        if (mime_type != nullptr) env->DeleteLocalRef(mime_type);
        if (kind != nullptr) env->DeleteLocalRef(kind);

        return false;
    }

    *out_descriptor = HostMediaAssetDescriptor {
        .id = string_ref_from_java(env, identifier, string_storage),
        .uri = string_ref_from_java(env, uri, string_storage),
        .filename = string_ref_from_java(env, filename, string_storage),
        .mime_type = string_ref_from_java(env, mime_type, string_storage),
        .kind = media_kind_from_enum(env, kind),
        .width = static_cast<uint32_t>(
            env->CallIntMethod(descriptor, media_asset_descriptor_get_width_method)
        ),
        .height = static_cast<uint32_t>(
            env->CallIntMethod(descriptor, media_asset_descriptor_get_height_method)
        ),
        .duration_ms = static_cast<uint64_t>(
            env->CallLongMethod(descriptor, media_asset_descriptor_get_duration_ms_method)
        ),
        .size_bytes = static_cast<uint64_t>(
            env->CallLongMethod(descriptor, media_asset_descriptor_get_size_bytes_method)
        ),
        .created_unix_ns = static_cast<uint64_t>(
            env->CallLongMethod(descriptor, media_asset_descriptor_get_created_unix_ns_method)
        ),
        .modified_unix_ns = static_cast<uint64_t>(
            env->CallLongMethod(descriptor, media_asset_descriptor_get_modified_unix_ns_method)
        ),
    };

    env->DeleteLocalRef(identifier);
    env->DeleteLocalRef(uri);
    env->DeleteLocalRef(filename);
    env->DeleteLocalRef(mime_type);
    env->DeleteLocalRef(kind);

    return !env->ExceptionCheck();
}

/// Decode one Java media page into one native page.
bool decode_media_page(
    JNIEnv *env,
    jobject page,
    HostMediaPage *out_page,
    std::deque<std::string> *string_storage,
    std::vector<HostMediaAssetDescriptor> *descriptor_storage
) {
    jobject assets = env->CallObjectMethod(page, media_list_result_get_assets_method);
    jstring next_cursor = reinterpret_cast<jstring>(
        env->CallObjectMethod(page, media_list_result_get_next_cursor_method)
    );
    jboolean has_more = env->CallBooleanMethod(page, media_list_result_get_has_more_method);

    if (assets == nullptr || next_cursor == nullptr) {
        if (assets != nullptr) env->DeleteLocalRef(assets);
        if (next_cursor != nullptr) env->DeleteLocalRef(next_cursor);

        return false;
    }

    jint asset_count = env->CallIntMethod(assets, list_size_method);
    descriptor_storage->reserve(static_cast<size_t>(asset_count));

    for (jint index = 0; index < asset_count; ++index) {
        jobject descriptor = env->CallObjectMethod(assets, list_get_method, index);
        if (descriptor == nullptr) {
            env->DeleteLocalRef(assets);
            env->DeleteLocalRef(next_cursor);
            return false;
        }

        HostMediaAssetDescriptor native_descriptor = {};
        bool did_decode = decode_media_descriptor(
            env,
            descriptor,
            &native_descriptor,
            string_storage
        );
        env->DeleteLocalRef(descriptor);
        if (!did_decode) {
            env->DeleteLocalRef(assets);
            env->DeleteLocalRef(next_cursor);
            return false;
        }

        descriptor_storage->push_back(native_descriptor);
    }

    out_page->assets = HostMediaAssetDescriptorSlice {
        .data = descriptor_storage->empty() ? nullptr : descriptor_storage->data(),
        .len = static_cast<uint32_t>(descriptor_storage->size()),
    };
    out_page->has_next_cursor = true;
    out_page->next_cursor = string_ref_from_java(env, next_cursor, string_storage);
    out_page->has_more = has_more == JNI_TRUE;

    env->DeleteLocalRef(assets);
    env->DeleteLocalRef(next_cursor);

    return !env->ExceptionCheck();
}

}

/// Resolve the media bridge methods from one runtime bridge instance.
bool resolve_media_methods(JNIEnv *env, jobject bridge) {
    return resolve_bridge_methods(env, bridge) && resolve_response_methods(env);
}

/// Call the media-list entrypoint on one registered bridge.
uint32_t call_media_list_for_session(
    uint64_t session_handle,
    HostMediaQuery query,
    HostMediaPage *output_page
) {
    if (output_page == nullptr) {
        return HOST_STATUS_FAILED;
    }

    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring cursor = query.has_cursor ? new_java_string(env, query.cursor) : nullptr;
    if (query.has_cursor && cursor == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jintArray kinds = new_media_kind_array(env, query.kinds);
    if (kinds == nullptr) {
        env->DeleteLocalRef(bridge);
        if (cursor != nullptr) {
            env->DeleteLocalRef(cursor);
        }
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        list_media_method,
        cursor,
        query.has_limit ? JNI_TRUE : JNI_FALSE,
        static_cast<jint>(query.limit),
        kinds,
        query.include_hidden ? JNI_TRUE : JNI_FALSE
    );
    env->DeleteLocalRef(bridge);
    if (cursor != nullptr) {
        env->DeleteLocalRef(cursor);
    }
    env->DeleteLocalRef(kinds);

    if (env->ExceptionCheck() || response == nullptr) {
        env->ExceptionClear();
        if (response != nullptr) {
            env->DeleteLocalRef(response);
        }
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, media_list_response_get_status_method);
    jobject page = env->CallObjectMethod(response, media_list_response_get_page_method);

    if (page != nullptr) {
        result_string_storage.clear();
        result_descriptor_storage.clear();

        if (!decode_media_page(
            env,
            page,
            output_page,
            &result_string_storage,
            &result_descriptor_storage
        )) {
            env->DeleteLocalRef(page);
            env->DeleteLocalRef(response);
            detach_jni_thread(did_attach_thread);
            return HOST_STATUS_FAILED;
        }
    } else {
        *output_page = HostMediaPage {};
    }

    if (page != nullptr) {
        env->DeleteLocalRef(page);
    }
    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the media-read entrypoint on one registered bridge.
uint32_t call_media_read_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostMediaAssetDescriptor *output_descriptor
) {
    if (output_descriptor == nullptr) {
        return HOST_STATUS_FAILED;
    }

    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier_string = new_java_string(env, identifier);
    if (identifier_string == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        read_media_method,
        identifier_string
    );
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_string);

    if (env->ExceptionCheck() || response == nullptr) {
        env->ExceptionClear();
        if (response != nullptr) {
            env->DeleteLocalRef(response);
        }
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, media_read_response_get_status_method);
    jobject descriptor = env->CallObjectMethod(response, media_read_response_get_descriptor_method);

    if (descriptor != nullptr) {
        result_string_storage.clear();
        bool did_decode = decode_media_descriptor(
            env,
            descriptor,
            output_descriptor,
            &result_string_storage
        );
        env->DeleteLocalRef(descriptor);
        if (!did_decode) {
            env->DeleteLocalRef(response);
            detach_jni_thread(did_attach_thread);
            return HOST_STATUS_FAILED;
        }
    } else {
        *output_descriptor = HostMediaAssetDescriptor {};
    }

    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the media-import entrypoint on one registered bridge.
uint32_t call_media_import_path_for_session(
    uint64_t session_handle,
    NativeStringRef path,
    int32_t kind,
    NativeStringRef *output_identifier
) {
    if (output_identifier == nullptr) {
        return HOST_STATUS_FAILED;
    }

    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring path_string = new_java_string(env, path);
    if (path_string == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        import_media_path_method,
        path_string,
        static_cast<jint>(kind)
    );
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(path_string);

    if (env->ExceptionCheck() || response == nullptr) {
        env->ExceptionClear();
        if (response != nullptr) {
            env->DeleteLocalRef(response);
        }
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, media_import_response_get_status_method);
    jstring identifier = reinterpret_cast<jstring>(
        env->CallObjectMethod(response, media_import_response_get_identifier_method)
    );

    result_string_storage.clear();
    if (identifier != nullptr) {
        *output_identifier = string_ref_from_java(env, identifier, &result_string_storage);
        env->DeleteLocalRef(identifier);
    } else {
        *output_identifier = NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the media-delete entrypoint on one registered bridge.
uint32_t call_media_delete_for_session(
    uint64_t session_handle,
    NativeStringSlice identifiers,
    uint32_t *deleted_count
) {
    if (deleted_count == nullptr) {
        return HOST_STATUS_FAILED;
    }

    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jobjectArray identifier_array = new_java_string_array(env, identifiers);
    if (identifier_array == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        delete_media_method,
        identifier_array
    );
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_array);

    if (env->ExceptionCheck() || response == nullptr) {
        env->ExceptionClear();
        if (response != nullptr) {
            env->DeleteLocalRef(response);
        }
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, media_delete_response_get_status_method);
    jint count = env->CallIntMethod(response, media_delete_response_get_deleted_count_method);
    *deleted_count = static_cast<uint32_t>(count);

    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
