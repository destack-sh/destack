import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testRuntimeBridgeRoutesDocumentRequestsIntoRuntimeHost() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings
  )
  let documentRequests = DocumentRequestRecorder()
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: documentRequests,
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let request = RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: 3),
    allowsMultipleSelection: true,
    contentTypes: ["image/png"]
  )

  try bridge.attach(runtimeHost: runtimeHost)
  let status = withBridgeDocumentRequest(request) { bridgeRequest in
    bindings.documentCallback!(sessionHandle.rawValue, bridgeRequest)
  }

  #expect(status == 0)
  #expect(documentRequests.requests == [request])
}

@MainActor
@Test
func testRuntimeBridgeSendsDocumentResultsIntoRuntimeIngress() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let result = RuntimeHostDocumentResult(
    requestID: HostRequestID(rawValue: 13),
    documents: [
      RuntimeHostDocumentDescriptor(
        uri: "file:///tmp/example.png",
        displayName: "example.png",
        contentType: "image/png"
      )
    ]
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.notifyDocumentResult(result)

  #expect(bindings.documentResults.count == 1)
  #expect(bindings.documentResults[0].0 == sessionHandle)
  #expect(bindings.documentResults[0].1 == result.requestID)
  #expect(bindings.documentResults[0].2 == result.documents)
}

@MainActor
@Test
func testRuntimeBridgeRejectsInvalidDocumentRequest() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )

  try bridge.attach(runtimeHost: runtimeHost)
  let invalidRequest = DestackRustDocumentRequest(
    request_id: 5,
    mime_types: DestackRustStringSlice(data: nil, len: 1),
    extensions: DestackRustStringSlice(data: nil, len: 0),
    allows_multiple_selection: false,
    allows_directory_selection: false,
    copies_to_sandbox: false
  )
  let status = bindings.documentCallback!(sessionHandle.rawValue, invalidRequest)

  #expect(status == 2)
}

@MainActor
@Test
func testRuntimeBridgeReportsMissingRuntimeHostAfterDetach() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let request = RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: 3),
    contentTypes: ["image/png"]
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.detach()
  let status = withBridgeDocumentRequest(request) { bridgeRequest in
    bindings.documentCallback!(sessionHandle.rawValue, bridgeRequest)
  }

  #expect(status == 3)
}
