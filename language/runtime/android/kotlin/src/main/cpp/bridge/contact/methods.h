#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_CONTACT_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_CONTACT_METHODS_H

#include "../types.h"

/// Resolve the contact bridge methods from one runtime bridge instance.
bool resolve_contact_methods(JNIEnv *env, jobject bridge);

/// Call the contact-list entrypoint on one registered bridge.
uint32_t call_contact_list_for_session(
    uint64_t session_handle,
    HostContactQuery query,
    HostContactPage *output_page
);

/// Call the contact-search entrypoint on one registered bridge.
uint32_t call_contact_search_for_session(
    uint64_t session_handle,
    NativeStringRef query_text,
    HostContactQuery query,
    HostContactPage *output_page
);

/// Call the contact-read entrypoint on one registered bridge.
uint32_t call_contact_read_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostContact *output_contact
);

/// Call the contact-create entrypoint on one registered bridge.
uint32_t call_contact_create_for_session(
    uint64_t session_handle,
    HostContactDraft draft,
    NativeStringRef *output_identifier
);

/// Call the contact-update entrypoint on one registered bridge.
uint32_t call_contact_update_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostContactDraft draft
);

/// Call the contact-delete entrypoint on one registered bridge.
uint32_t call_contact_delete_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
);

#endif
