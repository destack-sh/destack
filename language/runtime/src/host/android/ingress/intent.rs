use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostEvent, HostIntentEvent, HostIntentPayload};

/// Submit one Android open-url intent callback.
pub(crate) fn android_notify_intent_open_url(
    session_handle: HostSessionHandle,
    source: Option<&str>,
    url: &str,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenUrl {
            url: url.to_string(),
        },
    }));

    Ok(())
}

/// Submit one Android open-file intent callback.
pub(crate) fn android_notify_intent_open_file(
    session_handle: HostSessionHandle,
    source: Option<&str>,
    path: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::OpenFile {
            path: path.to_string(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one Android shared-text intent callback.
pub(crate) fn android_notify_intent_share_text(
    session_handle: HostSessionHandle,
    source: Option<&str>,
    text: &str,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareText {
            text: text.to_string(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one Android shared-file intent callback.
pub(crate) fn android_notify_intent_share_files(
    session_handle: HostSessionHandle,
    source: Option<&str>,
    paths: &[String],
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::ShareFiles {
            paths: paths.to_vec(),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}

/// Submit one Android custom-action intent callback.
pub(crate) fn android_notify_intent_custom_action(
    session_handle: HostSessionHandle,
    source: Option<&str>,
    action: &str,
    url: Option<&str>,
    paths: &[String],
    text: Option<&str>,
    mime_type: Option<&str>,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Intent(HostIntentEvent {
        source: source.map(str::to_string),
        payload: HostIntentPayload::CustomAction {
            action: action.to_string(),
            url: url.map(str::to_string),
            paths: paths.to_vec(),
            text: text.map(str::to_string),
            content_type: mime_type.map(str::to_string),
        },
    }));

    Ok(())
}
