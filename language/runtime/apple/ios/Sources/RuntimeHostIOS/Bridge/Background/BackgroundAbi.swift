import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one background status request.
typealias BackgroundStatusCallback =
  @convention(c) (UInt64, UnsafeMutablePointer<UInt32>?) -> UInt32

/// One C callback for one background task-list request.
typealias BackgroundListCallback =
  @convention(c) (UInt64, UnsafeMutablePointer<DestackRustBackgroundTaskDescriptorArray>?) -> UInt32

/// One C callback for one background register request.
typealias BackgroundRegisterCallback =
  @convention(c) (UInt64, DestackRustBackgroundTaskOptions) -> UInt32

/// One C callback for one background unregister request.
typealias BackgroundUnregisterCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// One C callback for one background trigger request.
typealias BackgroundTriggerTestCallback =
  @convention(c) (UInt64, DestackRustStringRef, UnsafeMutablePointer<Bool>?) -> UInt32

/// One C callback for one background completion request.
typealias BackgroundCompleteCallback =
  @convention(c) (UInt64, DestackRustStringRef, UInt32) -> UInt32

/// One low-level background ingress ABI for one iOS host bridge.
protocol BackgroundAbi {
  /// Deliver one background event into one runtime session.
  func notifyBackgroundEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostBackgroundEvent
  ) -> RuntimeAbiStatus
}

extension ProcessRuntimeAbi {
  /// Deliver one background event into one runtime session.
  func notifyBackgroundEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostBackgroundEvent
  ) -> RuntimeAbiStatus {
    withNativeBackgroundEvent(event) { nativeEvent in
      let status = destack_runtime_host_ios_notify_background_event(
        sessionHandle.rawValue,
        nativeEvent
      )

      return RuntimeAbiStatus(
        code: status.code,
        errorID: status.error_id
      )
    }
  }
}

/// Encode one Swift background event as one C bridge payload.
private func withNativeBackgroundEvent<T>(
  _ event: RuntimeHostBackgroundEvent,
  body: (DestackRustBackgroundEvent) -> T
) -> T {
  withNativeStringRef(event.metadata.identifier) { identifier in
    withNativeStringRef(event.metadata.executionID) { executionID in
      body(
        DestackRustBackgroundEvent(
          kind: encodeBackgroundEventKind(event.kind),
          metadata: DestackRustBackgroundEventMetadata(
            timestamp_ns: event.metadata.timestampNs,
            sequence: event.metadata.sequence,
            identifier: identifier,
            execution_id: executionID,
            deadline_unix_ns: event.metadata.deadlineUnixNs
          )
        )
      )
    }
  }
}

/// Encode one Swift background event kind as one C bridge payload.
private func encodeBackgroundEventKind(
  _ kind: RuntimeHostBackgroundEventKind
) -> UInt32 {
  switch kind {
  case .taskReady:
    return 1
  case .taskExpired:
    return 2
  }
}
