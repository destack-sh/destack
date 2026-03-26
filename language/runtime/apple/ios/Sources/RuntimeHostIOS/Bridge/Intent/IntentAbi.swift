import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one immediate intent can-open-url request.
typealias IntentCanOpenURLCallback =
  @convention(c) (UInt64, DestackRustStringRef, UnsafeMutablePointer<Bool>?) -> UInt32

/// One C callback for one immediate intent open-url request.
typealias IntentOpenURLCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// One C callback for one immediate intent open-path request.
typealias IntentOpenPathCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// One C callback for one immediate intent share-text request.
typealias IntentShareTextCallback =
  @convention(c) (UInt64, DestackRustStringRef, Bool, DestackRustStringRef) -> UInt32

/// One C callback for one immediate intent share-paths request.
typealias IntentSharePathsCallback =
  @convention(c) (UInt64, DestackRustStringSlice, Bool, DestackRustStringRef) -> UInt32

/// One low-level intent ingress ABI for one iOS host bridge.
protocol IntentAbi {
  /// Deliver one intent event into one runtime session.
  func notifyIntentEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostIntentEvent
  ) -> RuntimeAbiStatus
}

extension ProcessRuntimeAbi {
  /// Deliver one intent event into one runtime session.
  func notifyIntentEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostIntentEvent
  ) -> RuntimeAbiStatus {
    switch event.payload {
    case .openURL(let url):
      return withNativeStringRef(event.source) { source in
        withNativeStringRef(url) { url in
          let status = destack_runtime_host_ios_notify_intent_open_url(
            sessionHandle.rawValue,
            event.source != nil,
            source,
            url
          )

          return RuntimeAbiStatus(code: status.code, errorID: status.error_id)
        }
      }

    case .openFile(let path, let contentType):
      return withNativeStringRef(event.source) { source in
        withNativeStringRef(path) { path in
          withNativeStringRef(contentType) { contentType in
            let status = destack_runtime_host_ios_notify_intent_open_file(
              sessionHandle.rawValue,
              event.source != nil,
              source,
              path,
              contentType.data != nil,
              contentType
            )

            return RuntimeAbiStatus(code: status.code, errorID: status.error_id)
          }
        }
      }

    case .shareText(let text, let contentType):
      return withNativeStringRef(event.source) { source in
        withNativeStringRef(text) { text in
          withNativeStringRef(contentType) { contentType in
            let status = destack_runtime_host_ios_notify_intent_share_text(
              sessionHandle.rawValue,
              event.source != nil,
              source,
              text,
              contentType.data != nil,
              contentType
            )

            return RuntimeAbiStatus(code: status.code, errorID: status.error_id)
          }
        }
      }

    case .shareFiles(let paths, let contentType):
      return withNativeStringRef(event.source) { source in
        withNativeStringSlice(paths) { paths in
          withNativeStringRef(contentType) { contentType in
            let status = destack_runtime_host_ios_notify_intent_share_files(
              sessionHandle.rawValue,
              event.source != nil,
              source,
              paths,
              contentType.data != nil,
              contentType
            )

            return RuntimeAbiStatus(code: status.code, errorID: status.error_id)
          }
        }
      }

    case .customAction(let action, let url, let paths, let text, let contentType):
      return withNativeStringRef(event.source) { source in
        withNativeStringRef(action) { action in
          withNativeStringRef(url) { url in
            withNativeStringSlice(paths) { paths in
              withNativeStringRef(text) { text in
                withNativeStringRef(contentType) { contentType in
                  let status = destack_runtime_host_ios_notify_intent_custom_action(
                    sessionHandle.rawValue,
                    event.source != nil,
                    source,
                    action,
                    url.data != nil,
                    url,
                    paths,
                    text.data != nil,
                    text,
                    contentType.data != nil,
                    contentType
                  )

                  return RuntimeAbiStatus(code: status.code, errorID: status.error_id)
                }
              }
            }
          }
        }
      }
    }
  }
}
