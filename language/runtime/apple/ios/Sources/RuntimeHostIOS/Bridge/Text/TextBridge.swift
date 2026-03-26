import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let textOpenCallback: TextOpenCallback = {
  sessionHandle,
  request in
  handleTextOpen(
    sessionHandle: sessionHandle,
    request: request
  )
}

let textCloseCallback: TextCloseCallback = {
  sessionHandle,
  sessionID in
  handleTextClose(
    sessionHandle: sessionHandle,
    sessionID: sessionID
  )
}

let textSetGeometryCallback: TextGeometryCallback = {
  sessionHandle,
  request in
  handleTextSetGeometry(
    sessionHandle: sessionHandle,
    request: request
  )
}

let textSetStateCallback: TextStateCallback = {
  sessionHandle,
  request in
  handleTextSetState(
    sessionHandle: sessionHandle,
    request: request
  )
}

/// One text bridge lane for one attached iOS runtime host.
@MainActor
final class TextBridge: TextInputEvents {
  /// The runtime session routed through this text bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any TextAbi

  /// Create one text bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any TextAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// Send one host text-input event into the runtime ingress path.
  func sendTextInputEvent(
    _ event: RuntimeHostTextInputEvent
  ) {
    let status = bindings.notifyTextInputState(
      sessionHandle: sessionHandle,
      sessionID: event.sessionID,
      state: event.state
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver text-input state: code \(status.code), error \(status.errorID)"
    )
  }

  /// Open one iOS text session through the attached host.
  func openTextInput(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostTextInputOpenRequest
  ) -> UInt32 {
    runtimeHost.textInputRequests.openTextInput(request)
  }

  /// Close one iOS text session through the attached host.
  func closeTextInput(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostTextInputCloseRequest
  ) -> UInt32 {
    runtimeHost.textInputRequests.closeTextInput(request)
  }

  /// Update one iOS text geometry through the attached host.
  func setTextInputGeometry(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostTextInputGeometryRequest
  ) -> UInt32 {
    runtimeHost.textInputRequests.setTextInputGeometry(request)
  }

  /// Update one iOS text state through the attached host.
  func setTextInputState(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostTextInputStateRequest
  ) -> UInt32 {
    runtimeHost.textInputRequests.setTextInputState(request)
  }
}

/// Resolve one registered text bridge for one runtime session.
func guardTextBridge(
  sessionHandle: UInt64
) -> TextBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.textBridge
}

/// Handle one runtime callback asking to open one iOS text session.
private func handleTextOpen(
  sessionHandle: UInt64,
  request bridgeRequest: DestackRustTextOpenRequest
) -> UInt32 {
  guard let bridge = guardTextBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let request: RuntimeHostTextInputOpenRequest
  do {
    request = try decodeTextOpenRequest(bridgeRequest)
  } catch {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.openTextInput(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Handle one runtime callback asking to close one iOS text session.
private func handleTextClose(
  sessionHandle: UInt64,
  sessionID: UInt64
) -> UInt32 {
  guard let bridge = guardTextBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  return runOnMainThread {
    bridge.closeTextInput(
      runtimeHost: runtimeHost,
      RuntimeHostTextInputCloseRequest(sessionID: sessionID)
    )
  }
}

/// Handle one runtime callback asking to update one iOS text geometry.
private func handleTextSetGeometry(
  sessionHandle: UInt64,
  request bridgeRequest: DestackRustTextGeometryRequest
) -> UInt32 {
  guard let bridge = guardTextBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let request = RuntimeHostTextInputGeometryRequest(
    sessionID: bridgeRequest.session_id,
    geometry: RuntimeHostTextInputGeometry(
      localToTargetTransform: RuntimeHostTextInputTransform2D(
        xx: bridgeRequest.local_to_target_transform.xx,
        xy: bridgeRequest.local_to_target_transform.xy,
        yx: bridgeRequest.local_to_target_transform.yx,
        yy: bridgeRequest.local_to_target_transform.yy,
        tx: bridgeRequest.local_to_target_transform.tx,
        ty: bridgeRequest.local_to_target_transform.ty
      ),
      editorRectangle: RuntimeHostTextInputRectangle(
        x: bridgeRequest.editor_rectangle.x,
        y: bridgeRequest.editor_rectangle.y,
        width: bridgeRequest.editor_rectangle.width,
        height: bridgeRequest.editor_rectangle.height
      ),
      caretRectangle: bridgeRequest.has_caret_rectangle
        ? RuntimeHostTextInputRectangle(
          x: bridgeRequest.caret_rectangle.x,
          y: bridgeRequest.caret_rectangle.y,
          width: bridgeRequest.caret_rectangle.width,
          height: bridgeRequest.caret_rectangle.height
        )
        : nil,
      composingRectangle: bridgeRequest.has_composing_rectangle
        ? RuntimeHostTextInputRectangle(
          x: bridgeRequest.composing_rectangle.x,
          y: bridgeRequest.composing_rectangle.y,
          width: bridgeRequest.composing_rectangle.width,
          height: bridgeRequest.composing_rectangle.height
        )
        : nil
    )
  )

  return runOnMainThread {
    bridge.setTextInputGeometry(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Handle one runtime callback asking to update one iOS text state.
private func handleTextSetState(
  sessionHandle: UInt64,
  request bridgeRequest: DestackRustTextStateRequest
) -> UInt32 {
  guard let bridge = guardTextBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let state: RuntimeHostTextInputState
  do {
    state = try decodeTextState(bridgeRequest.state)
  } catch {
    return hostStatusInvalidArgument
  }
  let sessionID = bridgeRequest.session_id

  return runOnMainThread {
    bridge.setTextInputState(
      runtimeHost: runtimeHost,
      RuntimeHostTextInputStateRequest(
        sessionID: sessionID,
        state: state
      )
    )
  }
}

/// Decode one bridge open request into one Swift text request.
private func decodeTextOpenRequest(
  _ request: DestackRustTextOpenRequest
) throws -> RuntimeHostTextInputOpenRequest {
  guard let inputType = RuntimeHostTextInputType(rawValue: Int(request.config.input_type)) else {
    throw BridgeStringError.invalidStringSlice
  }

  return RuntimeHostTextInputOpenRequest(
    configuration: RuntimeHostTextInputConfiguration(
      sessionID: request.config.session_id,
      inputType: inputType,
      isMultiline: request.config.is_multiline,
      isSecure: request.config.is_secure
    ),
    state: try decodeTextState(request.state)
  )
}

/// Decode one bridge text state into one Swift value.
private func decodeTextState(
  _ state: DestackRustTextSessionState
) throws -> RuntimeHostTextInputState {
  RuntimeHostTextInputState(
    text: try tryDecodeNativeString(state.text),
    selection: decodeTextRange(state.selection),
    composing: state.has_composing ? decodeTextRange(state.composing) : nil
  )
}

/// Decode one bridge text range into one Swift value.
private func decodeTextRange(
  _ range: DestackRustTextRange
) -> RuntimeHostTextInputRange {
  RuntimeHostTextInputRange(
    startOffset: Int(range.start_offset),
    endOffset: Int(range.end_offset)
  )
}
