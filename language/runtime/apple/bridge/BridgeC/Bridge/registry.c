#include "Bridge/Types.h"
#include "Bridge/Registry.h"

#include <pthread.h>
#include <stddef.h>
#include <stdlib.h>

typedef struct CallbackEntry {
    uint64_t session_handle;
    DestackRustDocumentCallback document;
    DestackRustPermissionOpenSettingsCallback permission_open_settings;
    DestackRustPermissionRequestCallback permission_request;
    DestackRustIntentCanOpenUrlCallback intent_can_open_url;
    DestackRustIntentOpenUrlCallback intent_open_url;
    DestackRustIntentOpenPathCallback intent_open_path;
    DestackRustIntentShareTextCallback intent_share_text;
    DestackRustIntentSharePathsCallback intent_share_paths;
    uint32_t (*notification_post)(uint64_t session_handle, DestackRustNotificationRequest request);
    uint32_t (*notification_cancel)(uint64_t session_handle, DestackRustStringRef identifier);
    uint32_t (*notification_cancel_all)(uint64_t session_handle);
    struct CallbackEntry *next;
} CallbackEntry;

static pthread_mutex_t callback_lock = PTHREAD_MUTEX_INITIALIZER;
static CallbackEntry *callback_entries = NULL;

/// Return whether one callback entry still owns any live callbacks.
static bool is_callback_entry_empty(const CallbackEntry *entry) {
    return
        entry->document == NULL &&
        entry->permission_open_settings == NULL &&
        entry->permission_request == NULL &&
        entry->intent_can_open_url == NULL &&
        entry->intent_open_url == NULL &&
        entry->intent_open_path == NULL &&
        entry->intent_share_text == NULL &&
        entry->intent_share_paths == NULL &&
        entry->notification_post == NULL &&
        entry->notification_cancel == NULL &&
        entry->notification_cancel_all == NULL;
}

/// Resolve one callback entry for one runtime session.
static CallbackEntry *resolve_callback_entry(uint64_t session_handle) {
    CallbackEntry *entry = callback_entries;

    while (entry != NULL) {
        if (entry->session_handle == session_handle) {
            return entry;
        }

        entry = entry->next;
    }

    return NULL;
}

/// Create one callback entry when one session is first registered.
static CallbackEntry *ensure_callback_entry(uint64_t session_handle) {
    CallbackEntry *entry = resolve_callback_entry(session_handle);

    if (entry != NULL) {
        return entry;
    }

    entry = (CallbackEntry *)calloc(1, sizeof(CallbackEntry));
    if (entry == NULL) {
        return NULL;
    }

    entry->session_handle = session_handle;
    entry->next = callback_entries;
    callback_entries = entry;

    return entry;
}

/// Remove one empty callback entry for one runtime session.
static void remove_callback_entry_if_empty(uint64_t session_handle) {
    CallbackEntry *previous = NULL;
    CallbackEntry *entry = callback_entries;

    while (entry != NULL) {
        if (entry->session_handle == session_handle) {
            if (!is_callback_entry_empty(entry)) {
                return;
            }

            if (previous == NULL) {
                callback_entries = entry->next;
            } else {
                previous->next = entry->next;
            }

            free(entry);
            return;
        }

        previous = entry;
        entry = entry->next;
    }
}

/// Resolve the document callback for one runtime session.
DestackRustDocumentCallback resolve_document_callback(uint64_t session_handle) {
    DestackRustDocumentCallback callback = NULL;

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        callback = entry->document;
    }

    pthread_mutex_unlock(&callback_lock);

    return callback;
}

/// Store the document callback for one runtime session.
bool store_document_callback(
    uint64_t session_handle,
    DestackRustDocumentCallback callback
) {
    bool did_store = false;

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = ensure_callback_entry(session_handle);
    if (entry != NULL) {
        entry->document = callback;
        did_store = true;
    }

    pthread_mutex_unlock(&callback_lock);

    return did_store;
}

/// Clear the document callback for one runtime session.
void clear_document_callback(uint64_t session_handle) {
    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        entry->document = NULL;
        remove_callback_entry_if_empty(session_handle);
    }

    pthread_mutex_unlock(&callback_lock);
}

/// Resolve the permission callbacks for one runtime session.
PermissionCallbacks resolve_permission_callbacks(uint64_t session_handle) {
    PermissionCallbacks callbacks = {
        .open_settings = NULL,
        .request = NULL,
    };

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        callbacks.open_settings = entry->permission_open_settings;
        callbacks.request = entry->permission_request;
    }

    pthread_mutex_unlock(&callback_lock);

    return callbacks;
}

/// Store the permission callbacks for one runtime session.
bool store_permission_callbacks(
    uint64_t session_handle,
    DestackRustPermissionOpenSettingsCallback open_settings,
    DestackRustPermissionRequestCallback request
) {
    bool did_store = false;

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = ensure_callback_entry(session_handle);
    if (entry != NULL) {
        entry->permission_open_settings = open_settings;
        entry->permission_request = request;
        did_store = true;
    }

    pthread_mutex_unlock(&callback_lock);

    return did_store;
}

/// Clear the permission callbacks for one runtime session.
void clear_permission_callbacks(uint64_t session_handle) {
    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        entry->permission_open_settings = NULL;
        entry->permission_request = NULL;
        remove_callback_entry_if_empty(session_handle);
    }

    pthread_mutex_unlock(&callback_lock);
}

/// Resolve the intent callbacks for one runtime session.
IntentCallbacks resolve_intent_callbacks(uint64_t session_handle) {
    IntentCallbacks callbacks = {
        .can_open_url = NULL,
        .open_url = NULL,
        .open_path = NULL,
        .share_text = NULL,
        .share_paths = NULL,
    };

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        callbacks.can_open_url = entry->intent_can_open_url;
        callbacks.open_url = entry->intent_open_url;
        callbacks.open_path = entry->intent_open_path;
        callbacks.share_text = entry->intent_share_text;
        callbacks.share_paths = entry->intent_share_paths;
    }

    pthread_mutex_unlock(&callback_lock);

    return callbacks;
}

/// Store the intent callbacks for one runtime session.
bool store_intent_callbacks(
    uint64_t session_handle,
    DestackRustIntentCanOpenUrlCallback can_open_url,
    DestackRustIntentOpenUrlCallback open_url,
    DestackRustIntentOpenPathCallback open_path,
    DestackRustIntentShareTextCallback share_text,
    DestackRustIntentSharePathsCallback share_paths
) {
    bool did_store = false;

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = ensure_callback_entry(session_handle);
    if (entry != NULL) {
        entry->intent_can_open_url = can_open_url;
        entry->intent_open_url = open_url;
        entry->intent_open_path = open_path;
        entry->intent_share_text = share_text;
        entry->intent_share_paths = share_paths;
        did_store = true;
    }

    pthread_mutex_unlock(&callback_lock);

    return did_store;
}

/// Clear the intent callbacks for one runtime session.
void clear_intent_callbacks(uint64_t session_handle) {
    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        entry->intent_can_open_url = NULL;
        entry->intent_open_url = NULL;
        entry->intent_open_path = NULL;
        entry->intent_share_text = NULL;
        entry->intent_share_paths = NULL;
        remove_callback_entry_if_empty(session_handle);
    }

    pthread_mutex_unlock(&callback_lock);
}

/// Resolve the notification callbacks for one runtime session.
NotificationCallbacks resolve_notification_callbacks(uint64_t session_handle) {
    NotificationCallbacks callbacks = {
        .post = NULL,
        .cancel = NULL,
        .cancel_all = NULL,
    };

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        callbacks.post = entry->notification_post;
        callbacks.cancel = entry->notification_cancel;
        callbacks.cancel_all = entry->notification_cancel_all;
    }

    pthread_mutex_unlock(&callback_lock);

    return callbacks;
}

/// Store the notification callbacks for one runtime session.
bool store_notification_callbacks(
    uint64_t session_handle,
    uint32_t (*post)(uint64_t session_handle, DestackRustNotificationRequest request),
    uint32_t (*cancel)(uint64_t session_handle, DestackRustStringRef identifier),
    uint32_t (*cancel_all)(uint64_t session_handle)
) {
    bool did_store = false;

    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = ensure_callback_entry(session_handle);
    if (entry != NULL) {
        entry->notification_post = post;
        entry->notification_cancel = cancel;
        entry->notification_cancel_all = cancel_all;
        did_store = true;
    }

    pthread_mutex_unlock(&callback_lock);

    return did_store;
}

/// Clear the notification callbacks for one runtime session.
void clear_notification_callbacks(uint64_t session_handle) {
    pthread_mutex_lock(&callback_lock);

    CallbackEntry *entry = resolve_callback_entry(session_handle);
    if (entry != NULL) {
        entry->notification_post = NULL;
        entry->notification_cancel = NULL;
        entry->notification_cancel_all = NULL;
        remove_callback_entry_if_empty(session_handle);
    }

    pthread_mutex_unlock(&callback_lock);
}
