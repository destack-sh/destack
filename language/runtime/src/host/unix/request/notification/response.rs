use std::sync::Arc;

use notify_rust::{ActionResponse, CloseReason};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use tracing::warn;
use zbus::MatchRule;
use zbus::blocking::fdo::DBusProxy;
use zbus::blocking::{Connection, MessageIterator};
use zbus::message::Type as MessageType;

use crate::diagnostic::RuntimeResult;
use crate::host::{HostSessionId, Platform};
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationInteractedPayloadValue, NotificationRequestValue,
};
use crate::platform::os::notification::runtime;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, WorkerLoop};

/// The Unix notification response interface.
const UNIX_NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";

/// The Unix notification action signal member.
const UNIX_NOTIFICATION_ACTION_MEMBER: &str = "ActionInvoked";

/// The Unix notification close signal member.
const UNIX_NOTIFICATION_CLOSED_MEMBER: &str = "NotificationClosed";

/// Process-global Unix notification response service.
struct UnixNotificationResponseService {
    /// Active response routes keyed by runtime and server id.
    state: Arc<Mutex<UnixNotificationResponseState>>,
    /// Shared DBus response loop.
    _worker: WorkerLoop,
}

/// Mutable Unix notification response routes.
#[derive(Default)]
struct UnixNotificationResponseState {
    /// Active notifications grouped by runtime id and runtime notification id.
    runtimes: FxHashMap<HostSessionId, FxHashMap<String, u32>>,
    /// Active notifications keyed by notification server id.
    notifications: FxHashMap<u32, ActiveUnixNotification>,
}

/// One active Unix notification route.
#[derive(Clone)]
struct ActiveUnixNotification {
    /// Owning runtime for this notification.
    host_session_id: HostSessionId,
    /// Owning platform for emitted ingress events.
    platform: Platform,
    /// Runtime notification identifier.
    id: String,
    /// Original notification request payload.
    request: NotificationRequestValue,
}

impl UnixNotificationResponseService {
    /// Create one Unix notification response service.
    fn new() -> RuntimeResult<Self> {
        let state = Arc::new(Mutex::new(UnixNotificationResponseState::default()));
        let worker_state = Arc::clone(&state);
        let connection = unix_notification_signal_connection()?;
        let worker = WorkerLoop::open(
            "destack-notification-unix",
            "platform.service.spawn",
            Self::POLICY,
            move || {
                let shutdown_connection = connection.clone();

                Ok((
                    Box::new(move || {
                        let _ = shutdown_connection.close();
                    }),
                    Box::new(move || run_notification_response_loop(worker_state, connection)),
                ))
            },
        )?;

        Ok(Self {
            state,
            _worker: worker,
        })
    }

    /// Register one active Unix notification response route.
    fn register_notification(
        &self,
        host_session_id: HostSessionId,
        platform: Platform,
        id: String,
        request: NotificationRequestValue,
        server_id: u32,
    ) {
        let mut state = self.state.lock();
        let runtime_notifications = state.runtimes.entry(host_session_id).or_default();

        runtime_notifications.insert(id.clone(), server_id);
        state.notifications.insert(
            server_id,
            ActiveUnixNotification {
                host_session_id,
                platform,
                id,
                request,
            },
        );
    }

    /// Remove one active Unix notification route and return its server id.
    fn remove_notification(&self, host_session_id: HostSessionId, id: &str) -> Option<u32> {
        let mut state = self.state.lock();
        remove_notification_by_runtime_id(&mut state, host_session_id, id)
    }

    /// Remove every active Unix notification route for one runtime.
    fn unregister_runtime(&self, host_session_id: HostSessionId) {
        let mut state = self.state.lock();
        remove_notification_runtime(&mut state, host_session_id);
    }
}

impl Service for UnixNotificationResponseService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

/// Register one active Unix notification response route.
pub(super) fn register_active_notification(
    host_session_id: HostSessionId,
    platform: Platform,
    id: String,
    request: NotificationRequestValue,
    server_id: u32,
) -> RuntimeResult<()> {
    let service = unix_notification_response_service()?;

    service.register_notification(host_session_id, platform, id, request, server_id);

    Ok(())
}

/// Remove one active Unix notification route and return its server id.
pub(super) fn remove_active_notification(
    host_session_id: HostSessionId,
    id: &str,
) -> RuntimeResult<Option<u32>> {
    let service = unix_notification_response_service()?;

    Ok(service.remove_notification(host_session_id, id))
}

/// Remove every active Unix notification route for one runtime.
pub(super) fn unregister_notification_runtime(host_session_id: HostSessionId) {
    if let Some(service) = UnixNotificationResponseService::active() {
        service.unregister_runtime(host_session_id);
    }
}

/// Return the shared Unix notification response service.
fn unix_notification_response_service() -> RuntimeResult<Arc<UnixNotificationResponseService>> {
    UnixNotificationResponseService::global(UnixNotificationResponseService::new)
}

/// Create one DBus connection configured for Unix notification response signals.
fn unix_notification_signal_connection() -> RuntimeResult<Connection> {
    let connection = Connection::session().map_err(|error| {
        io_operation_error(
            "destack.os.notification.post",
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to open Unix notification session bus: {error}"),
        )
    })?;
    let proxy = DBusProxy::new(&connection).map_err(|error| {
        io_operation_error(
            "destack.os.notification.post",
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to create Unix notification DBus proxy: {error}"),
        )
    })?;

    for rule in [
        unix_notification_action_rule()?,
        unix_notification_closed_rule()?,
    ] {
        proxy.add_match_rule(rule).map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to register Unix notification signal rule: {error}"),
            )
        })?;
    }

    Ok(connection)
}

/// Build the Unix notification action signal rule.
fn unix_notification_action_rule() -> RuntimeResult<MatchRule<'static>> {
    MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(UNIX_NOTIFICATION_INTERFACE)
        .map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to build Unix notification action rule interface: {error}"),
            )
        })?
        .member(UNIX_NOTIFICATION_ACTION_MEMBER)
        .map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to build Unix notification action rule member: {error}"),
            )
        })?
        .build()
        .map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to build Unix notification action rule: {error}"),
            )
        })
}

/// Build the Unix notification closed signal rule.
fn unix_notification_closed_rule() -> RuntimeResult<MatchRule<'static>> {
    MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(UNIX_NOTIFICATION_INTERFACE)
        .map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to build Unix notification close rule interface: {error}"),
            )
        })?
        .member(UNIX_NOTIFICATION_CLOSED_MEMBER)
        .map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to build Unix notification close rule member: {error}"),
            )
        })?
        .build()
        .map_err(|error| {
            io_operation_error(
                "destack.os.notification.post",
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to build Unix notification close rule: {error}"),
            )
        })
}

/// Run the Unix notification response loop until the connection closes.
fn run_notification_response_loop(
    state: Arc<Mutex<UnixNotificationResponseState>>,
    connection: Connection,
) -> RuntimeResult<()> {
    let mut iterator = MessageIterator::from(&connection);

    while let Some(message) = iterator.next() {
        let Ok(message) = message else {
            continue;
        };

        match unix_notification_member(&message).as_deref() {
            Some(UNIX_NOTIFICATION_ACTION_MEMBER) => {
                let Ok((server_id, action_id)) = message.body().deserialize::<(u32, String)>()
                else {
                    continue;
                };

                handle_notification_response(
                    &state,
                    server_id,
                    ActionResponse::Custom(action_id.as_str()),
                );
            }
            Some(UNIX_NOTIFICATION_CLOSED_MEMBER) => {
                let Ok((server_id, reason)) = message.body().deserialize::<(u32, u32)>() else {
                    continue;
                };

                handle_notification_response(
                    &state,
                    server_id,
                    ActionResponse::Closed(reason.into()),
                );
            }
            _ => {}
        }
    }

    Ok(())
}

/// Handle one Unix notification action or close signal.
fn handle_notification_response(
    state: &Arc<Mutex<UnixNotificationResponseState>>,
    server_id: u32,
    response: ActionResponse<'_>,
) {
    let Some(notification) = remove_notification_by_server_id(state, server_id) else {
        return;
    };

    runtime::remove_posted_notification(notification.host_session_id, &notification.id);

    // publish the corresponding host event when the close reason maps to one surface event
    match response {
        ActionResponse::Custom(action_id) => {
            let sequence = runtime::next_notification_sequence(
                notification.host_session_id,
                notification.platform,
            );
            let payload = NotificationInteractedPayloadValue {
                action_id: Some(action_id.to_string()),
                action_response_text: None,
            };
            let publish_result = runtime::publish_interacted_notification(
                notification.host_session_id,
                notification.platform,
                notification.id.clone(),
                notification.request.clone(),
                sequence,
                payload,
            );

            if let Err(error) = publish_result {
                warn!(
                    ?error,
                    notification_id = notification.id,
                    "failed to publish Unix notification interaction"
                );
            }
        }

        ActionResponse::Closed(CloseReason::Dismissed) => {
            let sequence = runtime::next_notification_sequence(
                notification.host_session_id,
                notification.platform,
            );
            let publish_result = runtime::publish_dismissed_notification(
                notification.host_session_id,
                notification.platform,
                notification.id.clone(),
                notification.request.clone(),
                sequence,
            );

            if let Err(error) = publish_result {
                warn!(
                    ?error,
                    notification_id = notification.id,
                    "failed to publish Unix notification dismissal"
                );
            }
        }

        ActionResponse::Closed(CloseReason::Expired)
        | ActionResponse::Closed(CloseReason::CloseAction)
        | ActionResponse::Closed(CloseReason::Other(_)) => {}
    }
}

/// Remove one active notification by server id.
fn remove_notification_by_server_id(
    state: &Arc<Mutex<UnixNotificationResponseState>>,
    server_id: u32,
) -> Option<ActiveUnixNotification> {
    let mut state = state.lock();
    let notification = state.notifications.remove(&server_id)?;
    let mut remove_runtime = false;

    if let Some(runtime_notifications) = state.runtimes.get_mut(&notification.host_session_id) {
        runtime_notifications.remove(&notification.id);
        remove_runtime = runtime_notifications.is_empty();
    }

    if remove_runtime {
        state.runtimes.remove(&notification.host_session_id);
    }

    Some(notification)
}

/// Remove one active notification by runtime id and runtime notification id.
fn remove_notification_by_runtime_id(
    state: &mut UnixNotificationResponseState,
    host_session_id: HostSessionId,
    id: &str,
) -> Option<u32> {
    let mut remove_runtime = false;
    let server_id = state
        .runtimes
        .get_mut(&host_session_id)
        .and_then(|runtime_notifications| {
            let server_id = runtime_notifications.remove(id);
            remove_runtime = runtime_notifications.is_empty();
            server_id
        });

    if remove_runtime {
        state.runtimes.remove(&host_session_id);
    }

    if let Some(server_id) = server_id {
        state.notifications.remove(&server_id);
    }

    server_id
}

/// Remove every active notification for one runtime.
fn remove_notification_runtime(
    state: &mut UnixNotificationResponseState,
    host_session_id: HostSessionId,
) {
    let Some(runtime_notifications) = state.runtimes.remove(&host_session_id) else {
        return;
    };

    for server_id in runtime_notifications.into_values() {
        state.notifications.remove(&server_id);
    }
}

/// Return the message member string when present.
fn unix_notification_member(message: &zbus::Message) -> Option<String> {
    message
        .header()
        .member()
        .map(|member| member.as_str().to_string())
}
