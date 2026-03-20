use super::*;

/// Map one host lifecycle state to the public runtime lifecycle state.
pub(crate) fn lifecycle_state_from_host(state: HostLifecycleState) -> LifecycleState {
    match state {
        HostLifecycleState::Initializing => LifecycleState::Inactive,
        HostLifecycleState::Running => LifecycleState::Active,
        HostLifecycleState::Paused => LifecycleState::Inactive,
        HostLifecycleState::Stopped => LifecycleState::Background,
        HostLifecycleState::Destroyed => LifecycleState::Terminating,
    }
}

/// Return the lifecycle event kind for one host lifecycle transition.
pub(crate) fn lifecycle_event_kind_for_transition(
    previous: HostLifecycleState,
    next: HostLifecycleState,
) -> Option<&'static str> {
    match (previous, next) {
        (HostLifecycleState::Initializing, HostLifecycleState::Running) => Some("launch"),
        (HostLifecycleState::Paused, HostLifecycleState::Running) => Some("resume"),
        (HostLifecycleState::Stopped, HostLifecycleState::Running) => Some("foreground"),
        (HostLifecycleState::Running, HostLifecycleState::Paused) => Some("pause"),
        (HostLifecycleState::Running, HostLifecycleState::Stopped)
        | (HostLifecycleState::Paused, HostLifecycleState::Stopped) => Some("background"),
        (_, HostLifecycleState::Destroyed) => Some("terminate"),
        _ => None,
    }
}

/// Build one lifecycle event payload for one kind string.
pub(crate) fn lifecycle_event_for_kind(
    kind: &'static str,
    metadata: LifecycleEventMetadata,
) -> LifecycleEventValue {
    match kind {
        "launch" => LifecycleEventValue::LifecycleLaunchEvent(LifecycleLaunchEventValue {
            kind: "launch".to_string(),
            metadata,
        }),
        "resume" => LifecycleEventValue::LifecycleResumeEvent(LifecycleResumeEventValue {
            kind: "resume".to_string(),
            metadata,
        }),
        "pause" => LifecycleEventValue::LifecyclePauseEvent(LifecyclePauseEventValue {
            kind: "pause".to_string(),
            metadata,
        }),
        "background" => {
            LifecycleEventValue::LifecycleBackgroundEvent(LifecycleBackgroundEventValue {
                kind: "background".to_string(),
                metadata,
            })
        }
        "foreground" => {
            LifecycleEventValue::LifecycleForegroundEvent(LifecycleForegroundEventValue {
                kind: "foreground".to_string(),
                metadata,
            })
        }
        "terminate" => LifecycleEventValue::LifecycleTerminateEvent(LifecycleTerminateEventValue {
            kind: "terminate".to_string(),
            metadata,
        }),
        _ => unreachable!("unknown lifecycle event kind"),
    }
}

/// Return whether one stream accepts one queued intent payload.
pub(crate) fn stream_accepts_payload(
    options: IntentOpenOptions,
    payload: &IntentQueuedPayload,
) -> bool {
    match payload {
        IntentQueuedPayload::OpenUrl { .. } => options.include_open_url,
        IntentQueuedPayload::OpenFile { .. } => options.include_open_file,
        IntentQueuedPayload::ShareText { .. } | IntentQueuedPayload::ShareFiles { .. } => {
            options.include_share
        }
        IntentQueuedPayload::CustomAction { .. } => options.include_custom_action,
    }
}

/// Return whether one background stream accepts this background event.
pub(crate) fn stream_accepts_background_event(
    options: BackgroundEventOpenOptionsValue,
    event: &BackgroundEventValue,
) -> bool {
    match event {
        BackgroundEventValue::BackgroundTaskReadyEvent(_) => options.include_task_ready,
        BackgroundEventValue::BackgroundTaskExpiredEvent(_) => options.include_task_expired,
    }
}

/// Return whether one notification stream accepts this notification event.
pub(crate) fn stream_accepts_notification_event(
    options: NotificationEventOpenOptionsValue,
    event: &NotificationEventValue,
) -> bool {
    match event {
        NotificationEventValue::NotificationDeliveredEvent(_) => options.include_delivered,
        NotificationEventValue::NotificationInteractedEvent(_) => options.include_interacted,
        NotificationEventValue::NotificationDismissedEvent(_) => options.include_dismissed,
    }
}

/// Convert one host intent payload into runtime-owned queue data.
pub(crate) fn intent_payload_from_host(payload: &HostIntentPayload) -> IntentQueuedPayload {
    match payload {
        HostIntentPayload::OpenUrl { url } => IntentQueuedPayload::OpenUrl { url: url.clone() },
        HostIntentPayload::OpenFile { path, content_type } => IntentQueuedPayload::OpenFile {
            path: path.clone(),
            content_type: content_type.clone(),
        },
        HostIntentPayload::ShareText { text, content_type } => IntentQueuedPayload::ShareText {
            text: text.clone(),
            content_type: content_type.clone(),
        },
        HostIntentPayload::ShareFiles {
            paths,
            content_type,
        } => IntentQueuedPayload::ShareFiles {
            paths: paths.clone(),
            content_type: content_type.clone(),
        },
        HostIntentPayload::CustomAction {
            action,
            url,
            paths,
            text,
            content_type,
        } => IntentQueuedPayload::CustomAction {
            action: action.clone(),
            url: url.clone(),
            paths: paths.clone(),
            text: text.clone(),
            content_type: content_type.clone(),
        },
    }
}
