import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one deferred document-pick request.
typealias DocumentCallback =
  @convention(c) (UInt64, DestackRustDocumentRequest) -> UInt32

/// One low-level document lane ABI surface for one iOS host bridge.
protocol DocumentAbi {
  /// Deliver one document result into one runtime session.
  func notifyDocumentResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID,
    documents: [RuntimeHostDocumentDescriptor]
  ) -> RuntimeAbiStatus
}

extension ProcessRuntimeAbi {
  /// Deliver one document result into one runtime session.
  func notifyDocumentResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID,
    documents: [RuntimeHostDocumentDescriptor]
  ) -> RuntimeAbiStatus {
    withDocumentDescriptorSlice(documents) { descriptors in
      let status = destack_runtime_host_ios_notify_document_result(
        sessionHandle.rawValue,
        requestID.rawValue,
        descriptors
      )

      return RuntimeAbiStatus(
        code: status.code,
        errorID: status.error_id
      )
    }
  }
}

/// Build one document descriptor slice for one Swift document array.
private func withDocumentDescriptorSlice<T>(
  _ documents: [RuntimeHostDocumentDescriptor],
  body: (DestackRustDocumentDescriptorSlice) -> T
) -> T {
  let storage = documents.map(DocumentDescriptorStorage.init)
  let descriptors = storage.map(\.descriptor)

  return descriptors.withUnsafeBufferPointer { buffer in
    body(
      DestackRustDocumentDescriptorSlice(
        data: buffer.baseAddress,
        len: UInt32(buffer.count)
      )
    )
  }
}

/// One retained document descriptor payload for one bridge call.
private struct DocumentDescriptorStorage {
  /// The retained utf8 bytes for one descriptor field.
  let uriBytes: [UInt8]
  /// The retained utf8 bytes for one descriptor field.
  let nameBytes: [UInt8]
  /// The retained utf8 bytes for one optional descriptor field.
  let mimeTypeBytes: [UInt8]?
  /// The retained utf8 bytes for one optional descriptor field.
  let localPathBytes: [UInt8]?
  /// The bridged descriptor value.
  let descriptor: DestackRustDocumentDescriptor

  /// Create one retained bridge descriptor.
  init(
    _ document: RuntimeHostDocumentDescriptor
  ) {
    let uriBytes = Array(document.uri.utf8)
    let nameBytes = Array((document.displayName ?? "").utf8)
    let mimeTypeBytes = document.contentType.map { Array($0.utf8) }
    let localPathBytes = document.localPath.map { Array($0.utf8) }

    self.uriBytes = uriBytes
    self.nameBytes = nameBytes
    self.mimeTypeBytes = mimeTypeBytes
    self.localPathBytes = localPathBytes
    self.descriptor =
      uriBytes.withUnsafeBufferPointer { uri in
        nameBytes.withUnsafeBufferPointer { name in
          withOptionalStringRef(mimeTypeBytes) { mimeType in
            withOptionalStringRef(localPathBytes) { localPath in
              DestackRustDocumentDescriptor(
                uri: DestackRustStringRef(
                  data: uri.baseAddress,
                  len: UInt32(uri.count)
                ),
                name: DestackRustStringRef(
                  data: name.baseAddress,
                  len: UInt32(name.count)
                ),
                has_mime_type: mimeType.data != nil,
                mime_type: mimeType,
                has_size_bytes: false,
                size_bytes: 0,
                has_modified_unix_ns: false,
                modified_unix_ns: 0,
                is_directory: false,
                has_local_path: localPath.data != nil,
                local_path: localPath
              )
            }
          }
        }
      }
  }
}

/// Build one optional C string reference from retained utf8 bytes.
private func withOptionalStringRef<T>(
  _ value: [UInt8]?,
  body: (DestackRustStringRef) -> T
) -> T {
  guard let value else {
    return body(DestackRustStringRef(data: nil, len: 0))
  }

  return value.withUnsafeBufferPointer { buffer in
    body(
      DestackRustStringRef(
        data: buffer.baseAddress,
        len: UInt32(buffer.count)
      )
    )
  }
}
