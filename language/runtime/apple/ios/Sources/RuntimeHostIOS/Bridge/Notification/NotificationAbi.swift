import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One low-level notification ingress ABI for one iOS host bridge.
protocol NotificationAbi {
  /// Deliver one notification event into one runtime session.
  func notifyNotificationEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostNotificationEvent,
    sequence: UInt64,
    timestampNs: UInt64
  ) -> RuntimeAbiStatus
}

/// One C callback for one immediate notification post request.
typealias NotificationPostCallback =
  @convention(c) (UInt64, DestackRustNotificationRequest) -> UInt32

/// One C callback for one immediate notification cancel request.
typealias NotificationCancelCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// One C callback for one immediate notification cancel-all request.
typealias NotificationCancelAllCallback =
  @convention(c) (UInt64) -> UInt32

extension ProcessRuntimeAbi {
  /// Deliver one notification event into one runtime session.
  func notifyNotificationEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostNotificationEvent,
    sequence: UInt64,
    timestampNs: UInt64
  ) -> RuntimeAbiStatus {
    withNativeStringRef(event.request.identifier) { identifier in
      withNativeStringRef(event.request.title) { title in
        withNativeStringRef(event.request.body) { body in
          withNativeStringRef(event.actionIdentifier) { actionIdentifier in
            let kind: DestackRustNotificationEventKind

            switch event.kind {
            case .delivered:
              kind = DESTACK_RUST_NOTIFICATION_EVENT_DELIVERED
            case .activated:
              kind = DESTACK_RUST_NOTIFICATION_EVENT_ACTIVATED
            case .dismissed:
              kind = DESTACK_RUST_NOTIFICATION_EVENT_DISMISSED
            }

            let status = destack_runtime_host_ios_notify_notification_event(
              sessionHandle.rawValue,
              DestackRustNotificationEvent(
                kind: kind,
                sequence: sequence,
                timestamp_ns: timestampNs,
                request: DestackRustNotificationRequest(
                  identifier: identifier,
                  title: title,
                  body: body
                ),
                has_action_identifier: actionIdentifier.data != nil,
                action_identifier: actionIdentifier
              )
            )

            return RuntimeAbiStatus(
              code: status.code,
              errorID: status.error_id
            )
          }
        }
      }
    }
  }
}
