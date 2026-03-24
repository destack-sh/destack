#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_MEDIA_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_MEDIA_METHODS_H

#include "../types.h"

/// Resolve the media bridge methods from one runtime bridge instance.
bool resolve_media_methods(JNIEnv *env, jobject bridge);

/// Call the media-list entrypoint on one registered bridge.
uint32_t call_media_list_for_session(
    uint64_t session_handle,
    HostMediaQuery query,
    HostMediaPage *output_page
);

/// Call the media-read entrypoint on one registered bridge.
uint32_t call_media_read_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostMediaAssetDescriptor *output_descriptor
);

/// Call the media-import entrypoint on one registered bridge.
uint32_t call_media_import_path_for_session(
    uint64_t session_handle,
    NativeStringRef path,
    int32_t kind,
    NativeStringRef *output_identifier
);

/// Call the media-delete entrypoint on one registered bridge.
uint32_t call_media_delete_for_session(
    uint64_t session_handle,
    NativeStringSlice identifiers,
    uint32_t *deleted_count
);

#endif
