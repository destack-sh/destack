import Foundation
import RuntimeHostAppleBridgeTest
import RuntimeHostAppleCore

@testable import RuntimeHostIOS

extension ProcessRuntimeAbi {
  /// Open one live iOS host session for bridge tests.
  func openTestSession() -> HostSessionHandle {
    HostSessionHandle(rawValue: destack_runtime_host_ios_test_open_session())
  }

  /// Close one live iOS host session for bridge tests.
  func closeTestSession(
    sessionHandle: HostSessionHandle
  ) {
    destack_runtime_host_ios_test_close_session(sessionHandle.rawValue)
  }

  /// Submit one iOS document request through the live runtime host bridge.
  func submitTestDocumentRequest(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID,
    mimeTypes: [String],
    extensions: [String],
    allowsMultipleSelection: Bool,
    allowsDirectorySelection: Bool,
    copiesToSandbox: Bool
  ) -> HostAbiStatus {
    withNativeStringSlice(mimeTypes) { mimeTypes in
      withNativeStringSlice(extensions) { extensions in
        let request = DestackRustDocumentRequest(
          request_id: requestID.rawValue,
          mime_types: mimeTypes,
          extensions: extensions,
          allows_multiple_selection: allowsMultipleSelection,
          allows_directory_selection: allowsDirectorySelection,
          copies_to_sandbox: copiesToSandbox
        )

        return destack_runtime_host_ios_test_submit_document_request(
          sessionHandle.rawValue,
          request
        )
      }
    }
  }

  /// Submit one iOS permission request through the live runtime host bridge.
  func submitTestPermissionRequest(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID,
    permission: String
  ) -> HostAbiStatus {
    withNativeStringSlice([permission]) { permissions in
      let request = DestackRustPermissionRequest(
        request_id: requestID.rawValue,
        permissions: permissions
      )

      return destack_runtime_host_ios_test_submit_permission_request(
        sessionHandle.rawValue,
        request
      )
    }
  }

  /// Submit one iOS permission-settings request through the live runtime host bridge.
  func openTestPermissionSettings(
    sessionHandle: HostSessionHandle
  ) -> HostAbiStatus {
    destack_runtime_host_ios_test_open_permission_settings(sessionHandle.rawValue)
  }

  /// Query can-open-url through the live bridge test path.
  func testIntentCanOpenURL(
    sessionHandle: HostSessionHandle,
    url: String
  ) -> (HostAbiStatus, Bool) {
    var isSupported = false
    let status =
      url.utf8.withContiguousStorageIfAvailable { buffer in
        destack_runtime_host_ios_test_intent_can_open_url(
          sessionHandle.rawValue,
          DestackRustStringRef(
            data: buffer.baseAddress,
            len: UInt32(buffer.count)
          ),
          &isSupported
        )
      } ?? hostStatusFailed

    return (status, isSupported)
  }

  /// Submit one iOS notification request through the live runtime host bridge.
  func submitTestNotificationPost(
    sessionHandle: HostSessionHandle,
    request: RuntimeHostNotificationRequest
  ) -> HostAbiStatus {
    withNativeStringRef(request.identifier) { identifier in
      withNativeStringRef(request.title) { title in
        withNativeStringRef(request.body) { body in
          destack_runtime_host_ios_test_submit_notification_post(
            sessionHandle.rawValue,
            DestackRustNotificationRequest(
              identifier: identifier,
              title: title,
              body: body
            )
          )
        }
      }
    }
  }
}
