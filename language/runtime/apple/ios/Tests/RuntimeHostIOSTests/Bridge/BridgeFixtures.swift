import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

@testable import RuntimeHostIOS

func hasRuntimeBridgeLibrary() -> Bool {
  guard let libraryPath = ProcessInfo.processInfo.environment["DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY"]
  else {
    return false
  }

  return !libraryPath.isEmpty
}

func withBridgeDocumentRequest<T>(
  _ request: RuntimeHostDocumentRequest,
  body: (DestackRustDocumentRequest) -> T
) -> T {
  withNativeStringSlice(request.contentTypes) { mimeTypes in
    withNativeStringSlice([]) { extensions in
      body(
        DestackRustDocumentRequest(
          request_id: request.requestID.rawValue,
          mime_types: mimeTypes,
          extensions: extensions,
          allows_multiple_selection: request.allowsMultipleSelection,
          allows_directory_selection: false,
          copies_to_sandbox: false
        )
      )
    }
  }
}

func withBridgePermissionRequest<T>(
  _ request: RuntimeHostPermissionRequest,
  body: (DestackRustPermissionRequest) -> T
) -> T {
  withNativeStringSlice([request.permission]) { permissions in
    body(
      DestackRustPermissionRequest(
        request_id: request.requestID.rawValue,
        permissions: permissions
      )
    )
  }
}
