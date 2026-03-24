import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let intentCanOpenURLCallback: IntentCanOpenURLCallback = {
  sessionHandle,
  url,
  output in
  handleIntentCanOpenURL(
    sessionHandle: sessionHandle,
    url: url,
    output: output
  )
}

let intentOpenURLCallback: IntentOpenURLCallback = {
  sessionHandle,
  url in
  handleIntentOpenURL(
    sessionHandle: sessionHandle,
    url: url
  )
}

let intentOpenPathCallback: IntentOpenPathCallback = {
  sessionHandle,
  path in
  handleIntentOpenPath(
    sessionHandle: sessionHandle,
    path: path
  )
}

let intentShareTextCallback: IntentShareTextCallback = {
  sessionHandle,
  text,
  hasContentType,
  contentType in
  handleIntentShareText(
    sessionHandle: sessionHandle,
    text: text,
    hasContentType: hasContentType,
    contentType: contentType
  )
}

let intentSharePathsCallback: IntentSharePathsCallback = {
  sessionHandle,
  paths,
  hasContentType,
  contentType in
  handleIntentSharePaths(
    sessionHandle: sessionHandle,
    paths: paths,
    hasContentType: hasContentType,
    contentType: contentType
  )
}

/// One intent bridge lane for one attached iOS runtime host.
@MainActor
final class IntentBridge: IntentEvents {
  /// The runtime session routed through this intent bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any RuntimeAbi

  /// Create one intent bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any RuntimeAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// Send one intent event into the runtime ingress path.
  func sendIntentEvent(
    _ event: RuntimeHostIntentEvent
  ) {
    let status = bindings.notifyIntentEvent(
      sessionHandle: sessionHandle,
      event: event
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver intent event: code \(status.code), error \(status.errorID)"
    )
  }

  func canOpenURL(
    runtimeHost: RuntimeHost,
    _ url: String
  ) -> Bool {
    runtimeHost.intentRequests.canOpenURL(url)
  }

  func openURL(
    runtimeHost: RuntimeHost,
    _ url: String
  ) -> UInt32 {
    runtimeHost.intentRequests.openURL(url)
  }

  func openPath(
    runtimeHost: RuntimeHost,
    _ path: String
  ) -> UInt32 {
    runtimeHost.intentRequests.openPath(path)
  }

  func shareText(
    runtimeHost: RuntimeHost,
    _ text: String,
    contentType: String?
  ) -> UInt32 {
    runtimeHost.intentRequests.shareText(text, contentType: contentType)
  }

  func sharePaths(
    runtimeHost: RuntimeHost,
    _ paths: [String],
    contentType: String?
  ) -> UInt32 {
    runtimeHost.intentRequests.sharePaths(paths, contentType: contentType)
  }
}

/// Resolve one registered intent bridge for one runtime session.
func guardIntentBridge(
  sessionHandle: UInt64
) -> IntentBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.intentBridge
}

/// Handle one runtime callback asking whether one URL can be opened.
private func handleIntentCanOpenURL(
  sessionHandle: UInt64,
  url: DestackRustStringRef,
  output: UnsafeMutablePointer<Bool>?
) -> UInt32 {
  guard let bridge = guardIntentBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let url = decodeNativeString(url)
  let isSupported = runOnMainThread {
    bridge.canOpenURL(
      runtimeHost: runtimeHost,
      url
    )
  }

  output?.pointee = isSupported

  return hostStatusOk
}

/// Handle one runtime callback asking to open one URL.
private func handleIntentOpenURL(
  sessionHandle: UInt64,
  url: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardIntentBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let url = decodeNativeString(url)

  return runOnMainThread {
    bridge.openURL(
      runtimeHost: runtimeHost,
      url
    )
  }
}

/// Handle one runtime callback asking to open one path.
private func handleIntentOpenPath(
  sessionHandle: UInt64,
  path: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardIntentBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let path = decodeNativeString(path)

  return runOnMainThread {
    bridge.openPath(
      runtimeHost: runtimeHost,
      path
    )
  }
}

/// Handle one runtime callback asking to share one text payload.
private func handleIntentShareText(
  sessionHandle: UInt64,
  text: DestackRustStringRef,
  hasContentType: Bool,
  contentType: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardIntentBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let text = decodeNativeString(text)
  let contentType = hasContentType ? decodeNativeString(contentType) : nil

  return runOnMainThread {
    bridge.shareText(
      runtimeHost: runtimeHost,
      text,
      contentType: contentType
    )
  }
}

/// Handle one runtime callback asking to share one file-path list.
private func handleIntentSharePaths(
  sessionHandle: UInt64,
  paths: DestackRustStringSlice,
  hasContentType: Bool,
  contentType: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardIntentBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let paths = decodeNativeStringSlice(paths)
  let contentType = hasContentType ? decodeNativeString(contentType) : nil

  return runOnMainThread {
    bridge.sharePaths(
      runtimeHost: runtimeHost,
      paths,
      contentType: contentType
    )
  }
}
