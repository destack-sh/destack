import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let documentCallback: DocumentCallback = {
  sessionHandle,
  request in
  handleDocumentRequest(
    sessionHandle: sessionHandle,
    bridgeRequest: request
  )
}

/// One document bridge lane for one attached iOS runtime host.
@MainActor
final class DocumentBridge: DocumentEvents {
  /// The runtime session routed through this document bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any RuntimeAbi

  /// Create one document bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any RuntimeAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// Send one document result into the runtime ingress path.
  func sendDocumentResult(_ result: RuntimeHostDocumentResult) {
    let status = bindings.notifyDocumentResult(
      sessionHandle: sessionHandle,
      requestID: result.requestID,
      documents: result.documents
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver document result: code \(status.code), error \(status.errorID)"
    )
  }

  func submitRequest(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostDocumentRequest
  ) -> UInt32 {
    runtimeHost.documentRequests.submitDocumentRequest(request)

    return hostStatusOk
  }
}

/// Decode one document request from the runtime callback path and submit it through the attached runtime host.
private func handleDocumentRequest(
  sessionHandle: UInt64,
  bridgeRequest: DestackRustDocumentRequest
) -> UInt32 {
  let bridge = guardDocumentBridge(sessionHandle: sessionHandle)
  let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle)
  guard let bridge, let runtimeHost else {
    return hostStatusNotFound
  }

  let request: RuntimeHostDocumentRequest
  do {
    request = try decodeDocumentRequest(bridgeRequest)
  } catch {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.submitRequest(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Decode one typed bridge payload for the document bridge lane.
private func decodeDocumentRequest(
  _ request: DestackRustDocumentRequest
) throws -> RuntimeHostDocumentRequest {
  let mimeTypes = try tryDecodeNativeStringSlice(request.mime_types)
  let extensions = try tryDecodeNativeStringSlice(request.extensions)
  let contentTypes = mimeTypes + extensions.map { "application/x.destack-extension.\($0)" }

  return RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: request.request_id),
    allowsMultipleSelection: request.allows_multiple_selection,
    contentTypes: contentTypes
  )
}
