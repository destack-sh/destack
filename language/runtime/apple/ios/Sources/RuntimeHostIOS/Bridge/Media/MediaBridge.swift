import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let mediaListCallback: MediaListCallback = {
  sessionHandle,
  query,
  outputPage in
  handleMediaList(
    sessionHandle: sessionHandle,
    query: query,
    outputPage: outputPage
  )
}

let mediaReadCallback: MediaReadCallback = {
  sessionHandle,
  identifier,
  outputDescriptor in
  handleMediaRead(
    sessionHandle: sessionHandle,
    identifier: identifier,
    outputDescriptor: outputDescriptor
  )
}

let mediaImportPathCallback: MediaImportPathCallback = {
  sessionHandle,
  path,
  kind,
  outputIdentifier in
  handleMediaImportPath(
    sessionHandle: sessionHandle,
    path: path,
    kind: kind,
    outputIdentifier: outputIdentifier
  )
}

let mediaDeleteCallback: MediaDeleteCallback = {
  sessionHandle,
  identifiers,
  deletedCount in
  handleMediaDelete(
    sessionHandle: sessionHandle,
    identifiers: identifiers,
    deletedCount: deletedCount
  )
}

/// One temporary native allocation arena for one media bridge callback.
private final class MediaBridgeArena {
  /// The raw deallocation actions recorded for this callback.
  private var deallocations: [() -> Void] = []

  /// Remove every recorded allocation before one new callback payload is encoded.
  func reset() {
    for deallocate in deallocations.reversed() {
      deallocate()
    }

    deallocations.removeAll(keepingCapacity: true)
  }

  /// Release every recorded native allocation.
  deinit {
    for deallocate in deallocations.reversed() {
      deallocate()
    }
  }

  /// Allocate one copied UTF-8 buffer for one Swift string.
  func makeStringRef(
    _ value: String
  ) -> DestackRustStringRef {
    let bytes = Array(value.utf8)
    if bytes.isEmpty {
      return DestackRustStringRef(data: nil, len: 0)
    }

    let storage = UnsafeMutablePointer<UInt8>.allocate(capacity: bytes.count)
    storage.initialize(from: bytes, count: bytes.count)
    deallocations.append {
      storage.deinitialize(count: bytes.count)
      storage.deallocate()
    }

    return DestackRustStringRef(
      data: UnsafePointer(storage),
      len: UInt32(bytes.count)
    )
  }

  /// Allocate one copied native array and fill it with one builder closure.
  func makeArray<Element>(
    count: Int,
    fill: (UnsafeMutableBufferPointer<Element>) -> Void
  ) -> UnsafePointer<Element>? {
    if count == 0 {
      return nil
    }

    let storage = UnsafeMutablePointer<Element>.allocate(capacity: count)
    let buffer = UnsafeMutableBufferPointer(start: storage, count: count)
    fill(buffer)
    deallocations.append {
      storage.deinitialize(count: count)
      storage.deallocate()
    }

    return UnsafePointer(storage)
  }
}

/// Return the thread-local media bridge arena for one callback thread.
private func currentMediaBridgeArena() -> MediaBridgeArena {
  let dictionary = Thread.current.threadDictionary
  let key = "dev.destack.runtime.apple.media-bridge-arena"

  if let arena = dictionary[key] as? MediaBridgeArena {
    arena.reset()

    return arena
  }

  let arena = MediaBridgeArena()
  dictionary[key] = arena

  return arena
}

/// One media bridge lane for one attached iOS runtime host.
@MainActor
final class MediaBridge {
  /// The runtime session routed through this media bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any RuntimeAbi

  /// Create one media bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any RuntimeAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// List one page of media through the attached runtime host.
  func listMedia(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostMediaListRequest
  ) -> RuntimeHostMediaListResponse {
    return runtimeHost.mediaRequests.listMedia(request)
  }

  /// Read one media asset through the attached runtime host.
  func readMedia(
    runtimeHost: RuntimeHost,
    identifier: String
  ) -> RuntimeHostMediaReadResponse {
    return runtimeHost.mediaRequests.readMedia(identifier: identifier)
  }

  /// Import one path into the attached runtime host media library.
  func importMediaPath(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostMediaImportPathRequest
  ) -> RuntimeHostMediaImportPathResponse {
    return runtimeHost.mediaRequests.importMediaPath(request)
  }

  /// Delete one batch of media assets through the attached runtime host.
  func deleteMedia(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostMediaDeleteRequest
  ) -> RuntimeHostMediaDeleteResponse {
    return runtimeHost.mediaRequests.deleteMedia(request)
  }
}

/// Resolve one registered media bridge for one runtime session.
func guardMediaBridge(
  sessionHandle: UInt64
) -> MediaBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.mediaBridge
}

/// Handle one runtime callback asking to list one media page.
private func handleMediaList(
  sessionHandle: UInt64,
  query: DestackRustMediaQuery,
  outputPage: UnsafeMutablePointer<DestackRustMediaPage>?
) -> UInt32 {
  guard let bridge = guardMediaBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let outputPage else {
    return hostStatusInvalidArgument
  }

  let request = decodeMediaListRequest(query)
  let response = runOnMainThread {
    bridge.listMedia(
      runtimeHost: runtimeHost,
      request
    )
  }

  guard let page = response.page else {
    return response.status
  }

  return withEncodedMediaPage(page) { encodedPage in
    outputPage.pointee = encodedPage

    return response.status
  }
}

/// Handle one runtime callback asking to read one media asset.
private func handleMediaRead(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  outputDescriptor: UnsafeMutablePointer<DestackRustMediaAssetDescriptor>?
) -> UInt32 {
  guard let bridge = guardMediaBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let outputDescriptor else {
    return hostStatusInvalidArgument
  }

  let identifier = decodeNativeString(identifier)
  let response = runOnMainThread {
    bridge.readMedia(
      runtimeHost: runtimeHost,
      identifier: identifier
    )
  }

  guard let descriptor = response.descriptor else {
    return response.status
  }

  return withEncodedMediaDescriptor(descriptor) { encodedDescriptor in
    outputDescriptor.pointee = encodedDescriptor

    return response.status
  }
}

/// Handle one runtime callback asking to import one media path.
private func handleMediaImportPath(
  sessionHandle: UInt64,
  path: DestackRustStringRef,
  kind: Int32,
  outputIdentifier: UnsafeMutablePointer<DestackRustStringRef>?
) -> UInt32 {
  guard let bridge = guardMediaBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let outputIdentifier else {
    return hostStatusInvalidArgument
  }

  guard let mediaKind = decodeMediaKind(rawValue: kind) else {
    return hostStatusInvalidArgument
  }

  let request = RuntimeHostMediaImportPathRequest(
    path: decodeNativeString(path),
    kind: mediaKind
  )
  let response = runOnMainThread {
    bridge.importMediaPath(
      runtimeHost: runtimeHost,
      request
    )
  }

  guard let identifier = response.identifier else {
    return response.status
  }

  return withEncodedMediaString(identifier) { encodedIdentifier in
    outputIdentifier.pointee = encodedIdentifier

    return response.status
  }
}

/// Handle one runtime callback asking to delete media assets.
private func handleMediaDelete(
  sessionHandle: UInt64,
  identifiers: DestackRustStringSlice,
  deletedCount: UnsafeMutablePointer<UInt32>?
) -> UInt32 {
  guard let bridge = guardMediaBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let deletedCount else {
    return hostStatusInvalidArgument
  }

  let request = RuntimeHostMediaDeleteRequest(
    identifiers: decodeNativeStringSlice(identifiers)
  )
  let response = runOnMainThread {
    bridge.deleteMedia(
      runtimeHost: runtimeHost,
      request
    )
  }

  deletedCount.pointee = response.deletedCount

  return response.status
}

/// Decode one bridge media-list query into one runtime host request.
private func decodeMediaListRequest(
  _ query: DestackRustMediaQuery
) -> RuntimeHostMediaListRequest {
  RuntimeHostMediaListRequest(
    cursor: query.has_cursor ? decodeNativeString(query.cursor) : nil,
    limit: query.has_limit ? query.limit : nil,
    kinds: decodeMediaKinds(query.kinds),
    includeHidden: query.include_hidden
  )
}

/// Decode one bridge media kind into one runtime host media kind.
private func decodeMediaKind(
  _ rawValue: DestackRustMediaAssetKind
) -> RuntimeHostMediaAssetKind? {
  switch rawValue {
  case DESTACK_RUST_MEDIA_ASSET_KIND_IMAGE:
    .image
  case DESTACK_RUST_MEDIA_ASSET_KIND_VIDEO:
    .video
  case DESTACK_RUST_MEDIA_ASSET_KIND_AUDIO:
    .audio
  case DESTACK_RUST_MEDIA_ASSET_KIND_OTHER:
    .other
  default:
    nil
  }
}

/// Decode one raw bridge media kind into one runtime host media kind.
private func decodeMediaKind(
  rawValue: Int32
) -> RuntimeHostMediaAssetKind? {
  switch rawValue {
  case Int32(DESTACK_RUST_MEDIA_ASSET_KIND_IMAGE.rawValue):
    .image
  case Int32(DESTACK_RUST_MEDIA_ASSET_KIND_VIDEO.rawValue):
    .video
  case Int32(DESTACK_RUST_MEDIA_ASSET_KIND_AUDIO.rawValue):
    .audio
  case Int32(DESTACK_RUST_MEDIA_ASSET_KIND_OTHER.rawValue):
    .other
  default:
    nil
  }
}

/// Decode one bridge media-kind slice into one runtime host media-kind array.
private func decodeMediaKinds(
  _ values: DestackRustMediaAssetKindSlice
) -> [RuntimeHostMediaAssetKind] {
  guard values.len != 0, let data = values.data else {
    return []
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return buffer.compactMap { value in
    decodeMediaKind(value)
  }
}

/// Encode one runtime host media descriptor into one bridge descriptor.
private func encodeMediaDescriptor(
  _ descriptor: RuntimeHostMediaAssetDescriptor,
  arena: MediaBridgeArena
) -> DestackRustMediaAssetDescriptor {
  let identifier = arena.makeStringRef(descriptor.identifier)
  let uri = arena.makeStringRef(descriptor.uri)
  let filename = arena.makeStringRef(descriptor.filename)
  let mimeType = arena.makeStringRef(descriptor.mimeType)

  return DestackRustMediaAssetDescriptor(
    id: identifier,
    uri: uri,
    filename: filename,
    mime_type: mimeType,
    kind: encodeMediaKind(descriptor.kind),
    width: descriptor.width,
    height: descriptor.height,
    duration_ms: descriptor.durationMs,
    size_bytes: descriptor.sizeBytes,
    created_unix_ns: descriptor.createdUnixNs,
    modified_unix_ns: descriptor.modifiedUnixNs
  )
}

/// Encode one runtime host media page into one bridge page.
private func encodeMediaPage(
  _ page: RuntimeHostMediaListResult,
  arena: MediaBridgeArena
) -> DestackRustMediaPage {
  let assets = arena.makeArray(count: page.assets.count) { buffer in
    for (index, asset) in page.assets.enumerated() {
      buffer[index] = encodeMediaDescriptor(asset, arena: arena)
    }
  }
  let nextCursor = arena.makeStringRef(page.nextCursor)

  return DestackRustMediaPage(
    assets: DestackRustMediaAssetDescriptorSlice(
      data: assets,
      len: UInt32(page.assets.count)
    ),
    has_next_cursor: !page.nextCursor.isEmpty,
    next_cursor: nextCursor,
    has_more: page.hasMore
  )
}

/// Encode one media string for one immediate callback result.
private func withEncodedMediaString<T>(
  _ value: String,
  body: (DestackRustStringRef) -> T
) -> T {
  let arena = currentMediaBridgeArena()
  let encodedValue = arena.makeStringRef(value)

  return body(encodedValue)
}

/// Encode one media descriptor for one immediate callback result.
private func withEncodedMediaDescriptor<T>(
  _ descriptor: RuntimeHostMediaAssetDescriptor,
  body: (DestackRustMediaAssetDescriptor) -> T
) -> T {
  let arena = currentMediaBridgeArena()
  let encodedDescriptor = encodeMediaDescriptor(descriptor, arena: arena)

  return body(encodedDescriptor)
}

/// Encode one media page for one immediate callback result.
private func withEncodedMediaPage<T>(
  _ page: RuntimeHostMediaListResult,
  body: (DestackRustMediaPage) -> T
) -> T {
  let arena = currentMediaBridgeArena()
  let encodedPage = encodeMediaPage(page, arena: arena)

  return body(encodedPage)
}

/// Encode one runtime host media kind into one bridge media kind.
private func encodeMediaKind(
  _ kind: RuntimeHostMediaAssetKind
) -> DestackRustMediaAssetKind {
  switch kind {
  case .image:
    DESTACK_RUST_MEDIA_ASSET_KIND_IMAGE
  case .video:
    DESTACK_RUST_MEDIA_ASSET_KIND_VIDEO
  case .audio:
    DESTACK_RUST_MEDIA_ASSET_KIND_AUDIO
  case .other:
    DESTACK_RUST_MEDIA_ASSET_KIND_OTHER
  }
}
