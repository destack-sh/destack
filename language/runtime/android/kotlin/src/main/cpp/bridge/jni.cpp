#include "types.h"
#include "jni.h"
#include "registry.h"

#include <string>

namespace {

JavaVM *java_vm = nullptr;
jclass string_class = nullptr;

/// Decode one UTF-8 scalar at one byte offset.
bool decode_utf8_scalar(
    const uint8_t *data,
    uint32_t length,
    uint32_t *in_out_index,
    uint32_t *out_scalar
) {
    uint32_t index = *in_out_index;
    if (index >= length) {
        return false;
    }

    uint8_t first = data[index];

    // single byte
    if (first < 0x80) {
        *out_scalar = first;
        *in_out_index = index + 1;
        return true;
    }

    // two bytes
    if ((first & 0xE0) == 0xC0) {
        if (index + 1 >= length) {
            return false;
        }

        uint8_t second = data[index + 1];
        if ((second & 0xC0) != 0x80) {
            return false;
        }

        uint32_t scalar = ((first & 0x1F) << 6) | (second & 0x3F);
        if (scalar < 0x80) {
            return false;
        }

        *out_scalar = scalar;
        *in_out_index = index + 2;
        return true;
    }

    // three bytes
    if ((first & 0xF0) == 0xE0) {
        if (index + 2 >= length) {
            return false;
        }

        uint8_t second = data[index + 1];
        uint8_t third = data[index + 2];
        if ((second & 0xC0) != 0x80 || (third & 0xC0) != 0x80) {
            return false;
        }

        uint32_t scalar =
            ((first & 0x0F) << 12) |
            ((second & 0x3F) << 6) |
            (third & 0x3F);
        if (scalar < 0x800 || (scalar >= 0xD800 && scalar <= 0xDFFF)) {
            return false;
        }

        *out_scalar = scalar;
        *in_out_index = index + 3;
        return true;
    }

    // four bytes
    if ((first & 0xF8) == 0xF0) {
        if (index + 3 >= length) {
            return false;
        }

        uint8_t second = data[index + 1];
        uint8_t third = data[index + 2];
        uint8_t fourth = data[index + 3];
        if (
            (second & 0xC0) != 0x80 ||
            (third & 0xC0) != 0x80 ||
            (fourth & 0xC0) != 0x80
        ) {
            return false;
        }

        uint32_t scalar =
            ((first & 0x07) << 18) |
            ((second & 0x3F) << 12) |
            ((third & 0x3F) << 6) |
            (fourth & 0x3F);
        if (scalar < 0x10000 || scalar > 0x10FFFF) {
            return false;
        }

        *out_scalar = scalar;
        *in_out_index = index + 4;
        return true;
    }

    return false;
}

/// Append one Unicode scalar to one UTF-16 buffer.
void append_utf16_scalar(std::u16string *utf16, uint32_t scalar) {
    if (scalar <= 0xFFFF) {
        utf16->push_back(static_cast<char16_t>(scalar));
        return;
    }

    uint32_t offset = scalar - 0x10000;
    utf16->push_back(static_cast<char16_t>(0xD800 + (offset >> 10)));
    utf16->push_back(static_cast<char16_t>(0xDC00 + (offset & 0x3FF)));
}

/// Decode one native UTF-8 string into one UTF-16 buffer.
bool decode_utf16_string(NativeStringRef value, std::u16string *out_utf16) {
    out_utf16->clear();
    out_utf16->reserve(static_cast<size_t>(value.len));

    uint32_t index = 0;
    while (index < value.len) {
        uint32_t scalar = 0;
        if (!decode_utf8_scalar(value.data, value.len, &index, &scalar)) {
            return false;
        }

        append_utf16_scalar(out_utf16, scalar);
    }

    return true;
}

}

/// Resolve one JNI environment and report whether this call attached the thread.
bool resolve_jni_env(JNIEnv **out_env, bool *out_did_attach_thread) {
    if (java_vm == nullptr) {
        return false;
    }

    *out_env = nullptr;
    *out_did_attach_thread = false;

    jint get_env_status = java_vm->GetEnv(reinterpret_cast<void **>(out_env), JNI_VERSION_1_6);
    if (get_env_status == JNI_OK && *out_env != nullptr) {
        return true;
    }

    if (get_env_status != JNI_EDETACHED) {
        return false;
    }

#if defined(__ANDROID__)
    jint attach_status = java_vm->AttachCurrentThread(out_env, nullptr);
#else
    jint attach_status = java_vm->AttachCurrentThread(reinterpret_cast<void **>(out_env), nullptr);
#endif
    if (attach_status != JNI_OK || *out_env == nullptr) {
        return false;
    }

    *out_did_attach_thread = true;

    return true;
}

/// Detach one JNI thread when this callback attached it.
void detach_jni_thread(bool did_attach_thread) {
    if (!did_attach_thread || java_vm == nullptr) {
        return;
    }

    java_vm->DetachCurrentThread();
}

/// Build one Java string from one native string reference.
jstring new_java_string(JNIEnv *env, NativeStringRef value) {
    std::u16string utf16;
    if (!decode_utf16_string(value, &utf16)) {
        return nullptr;
    }

    return env->NewString(
        reinterpret_cast<const jchar *>(utf16.data()),
        static_cast<jsize>(utf16.size())
    );
}

/// Build one Java string array from one native string slice.
jobjectArray new_java_string_array(JNIEnv *env, NativeStringSlice values) {
    if (string_class == nullptr) {
        jclass local_string_class = env->FindClass("java/lang/String");
        if (local_string_class == nullptr) {
            return nullptr;
        }

        string_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_string_class));
        env->DeleteLocalRef(local_string_class);
        if (string_class == nullptr) {
            return nullptr;
        }
    }

    auto *slice = values.data;
    auto count = static_cast<jsize>(values.len);
    jobjectArray array = env->NewObjectArray(count, string_class, nullptr);
    if (array == nullptr) {
        return nullptr;
    }

    for (jsize index = 0; index < count; index += 1) {
        jstring element = new_java_string(env, slice[index]);
        if (element == nullptr) {
            env->DeleteLocalRef(array);
            return nullptr;
        }

        env->SetObjectArrayElement(array, index, element);
        env->DeleteLocalRef(element);
    }

    return array;
}

/// Register one native method table on one JVM class.
bool register_native_methods(
    JNIEnv *env,
    const char *class_name,
    JNINativeMethod *methods,
    jint count
) {
    jclass local_class = env->FindClass(class_name);
    if (local_class == nullptr) {
        return false;
    }

    jint status = env->RegisterNatives(local_class, methods, count);
    env->DeleteLocalRef(local_class);

    return status == JNI_OK;
}

/// Call one bridge method with one byte payload.
uint32_t call_bridge_bytes(
    JNIEnv *env,
    uint64_t session_handle,
    NativeSlice payload,
    jmethodID method
) {
    jobject bridge = resolve_bridge(env, session_handle);

    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jbyteArray payload_array = env->NewByteArray(static_cast<jsize>(payload.len));
    if (payload_array == nullptr) {
        return HOST_STATUS_FAILED;
    }

    if (payload.len != 0) {
        env->SetByteArrayRegion(
            payload_array,
            0,
            static_cast<jsize>(payload.len),
            reinterpret_cast<const jbyte *>(payload.data)
        );
    }

    jint status = env->CallIntMethod(bridge, method, payload_array);
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(payload_array);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Encode one runtime status into one long array for JNI.
jlongArray runtime_status_array(JNIEnv *env, RuntimeStatus status) {
    jlong values[2] = {
        static_cast<jlong>(status.code),
        static_cast<jlong>(status.error_id),
    };
    jlongArray result = env->NewLongArray(2);
    if (result == nullptr) {
        return nullptr;
    }

    env->SetLongArrayRegion(result, 0, 2, values);

    return result;
}

/// Capture the Java VM for later runtime callback trampolines.
extern "C" JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void * /* reserved */) {
    java_vm = vm;

    JNIEnv *env = nullptr;
    if (vm->GetEnv(reinterpret_cast<void **>(&env), JNI_VERSION_1_6) != JNI_OK || env == nullptr) {
        return JNI_ERR;
    }

    if (!register_runtime_ingress_natives(env)) {
        return JNI_ERR;
    }

    if (!register_runtime_session_natives(env)) {
        return JNI_ERR;
    }

    return JNI_VERSION_1_6;
}
