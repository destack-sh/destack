import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one immediate text-session open request.
typealias TextOpenCallback =
  @convention(c) (UInt64, DestackRustTextOpenRequest) -> UInt32

/// One C callback for one immediate text-session close request.
typealias TextCloseCallback =
  @convention(c) (UInt64, UInt64) -> UInt32

/// One C callback for one immediate text-session geometry update.
typealias TextGeometryCallback =
  @convention(c) (UInt64, DestackRustTextGeometryRequest) -> UInt32

/// One C callback for one immediate text-session state update.
typealias TextStateCallback =
  @convention(c) (UInt64, DestackRustTextStateRequest) -> UInt32

/// One low-level text ingress ABI for one iOS host bridge.
protocol TextAbi {
  /// Deliver one text-session state event into one runtime session.
  func notifyTextInputState(
    sessionHandle: HostSessionHandle,
    sessionID: UInt64,
    state: RuntimeHostTextInputState
  ) -> RuntimeAbiStatus
}

extension ProcessRuntimeAbi {
  /// Deliver one text-session state event into one runtime session.
  func notifyTextInputState(
    sessionHandle: HostSessionHandle,
    sessionID: UInt64,
    state: RuntimeHostTextInputState
  ) -> RuntimeAbiStatus {
    withNativeTextSessionState(state) { nativeState in
      let status = destack_runtime_host_ios_notify_text_input_state(
        sessionHandle.rawValue,
        sessionID,
        nativeState
      )

      return RuntimeAbiStatus(
        code: status.code,
        errorID: status.error_id
      )
    }
  }
}

/// Encode one Swift text-session state as one C bridge payload.
private func withNativeTextSessionState<T>(
  _ state: RuntimeHostTextInputState,
  body: (DestackRustTextSessionState) -> T
) -> T {
  withNativeStringRef(state.text) { text in
    body(
      DestackRustTextSessionState(
        text: text,
        selection: encodeTextRange(state.selection),
        has_composing: state.composing != nil,
        composing: encodeTextRange(
          state.composing
            ?? RuntimeHostTextInputRange(
              startOffset: 0,
              endOffset: 0
            )
        )
      )
    )
  }
}

/// Encode one Swift text range as one C bridge payload.
private func encodeTextRange(
  _ range: RuntimeHostTextInputRange
) -> DestackRustTextRange {
  DestackRustTextRange(
    start_offset: UInt32(range.startOffset),
    end_offset: UInt32(range.endOffset)
  )
}
