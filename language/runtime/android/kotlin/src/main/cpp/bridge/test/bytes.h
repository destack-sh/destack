#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_TESTING_BYTES_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_TESTING_BYTES_H

#include "../types.h"

#include <vector>

/// Read one JNI byte array into one owned native byte buffer.
std::vector<uint8_t> read_byte_array(JNIEnv *env, jbyteArray payload);

#endif
