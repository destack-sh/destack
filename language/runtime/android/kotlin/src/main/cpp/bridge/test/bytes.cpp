#include "../types.h"
#include "bytes.h"

#include <vector>

/// Read one JNI byte array into one owned native byte buffer.
std::vector<uint8_t> read_byte_array(JNIEnv *env, jbyteArray payload) {
    jsize payload_length = env->GetArrayLength(payload);
    std::vector<uint8_t> payload_bytes(static_cast<size_t>(payload_length));
    if (payload_length == 0) {
        return payload_bytes;
    }

    env->GetByteArrayRegion(
        payload,
        0,
        payload_length,
        reinterpret_cast<jbyte *>(payload_bytes.data())
    );

    return payload_bytes;
}
