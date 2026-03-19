use tracing::warn;

use notify_rust::{ActionResponse, CloseReason};

use crate::host::Platform;
use crate::host::app::notification::runtime;
use crate::host::core::HostRuntimeId;
use crate::platform::os::abi_generated::{
    NotificationInteractedPayloadValue, NotificationRequestValue,
};

use super::core::active_notification_registry;

/// Spawn one detached worker that waits for one Unix notification response.
pub(super) fn spawn_notification_response_worker(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    id: String,
    request: NotificationRequestValue,
    server_id: u32,
) {
    let thread_name = format!("notification-unix-{server_id}");

    let spawn_result = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            notify_rust::handle_action(server_id, |response| {
                handle_notification_response(host_runtime_id, platform, &id, request, response);
            });
        });

    // report worker startup failure loudly because response routing would otherwise be lost
    if let Err(error) = spawn_result {
        warn!(
            ?error,
            server_id, "failed to spawn Unix notification response worker"
        );
    }
}

/// Handle one Unix notification action or close signal.
fn handle_notification_response(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    id: &str,
    request: NotificationRequestValue,
    response: &ActionResponse<'_>,
) {
    unregister_active_notification(host_runtime_id, id);
    runtime::remove_posted_notification(host_runtime_id, id);

    // publish the corresponding host event when the close reason maps to one surface event
    match response {
        ActionResponse::Custom(action_id) => {
            let sequence = runtime::next_notification_sequence(host_runtime_id, platform);
            let payload = NotificationInteractedPayloadValue {
                action_id: Some((*action_id).to_string()),
                action_response_text: None,
            };
            let publish_result = runtime::publish_interacted_notification(
                host_runtime_id,
                platform,
                id.to_string(),
                request.clone(),
                sequence,
                payload,
            );

            if let Err(error) = publish_result {
                warn!(
                    ?error,
                    notification_id = id,
                    "failed to publish Unix notification interaction"
                );
            }
        }

        ActionResponse::Closed(CloseReason::Dismissed) => {
            let sequence = runtime::next_notification_sequence(host_runtime_id, platform);
            let publish_result = runtime::publish_dismissed_notification(
                host_runtime_id,
                platform,
                id.to_string(),
                request.clone(),
                sequence,
            );

            if let Err(error) = publish_result {
                warn!(
                    ?error,
                    notification_id = id,
                    "failed to publish Unix notification dismissal"
                );
            }
        }

        ActionResponse::Closed(CloseReason::Expired)
        | ActionResponse::Closed(CloseReason::CloseAction)
        | ActionResponse::Closed(CloseReason::Other(_)) => {}
    }
}

/// Remove one active Unix notification mapping.
fn unregister_active_notification(host_runtime_id: HostRuntimeId, id: &str) {
    let registry = active_notification_registry();
    let mut registry = registry.lock();
    let mut remove_runtime = false;

    if let Some(runtime_notifications) = registry.runtimes.get_mut(&host_runtime_id) {
        runtime_notifications.remove(id);

        if runtime_notifications.is_empty() {
            remove_runtime = true;
        }
    }

    if remove_runtime {
        registry.runtimes.remove(&host_runtime_id);
    }
}
