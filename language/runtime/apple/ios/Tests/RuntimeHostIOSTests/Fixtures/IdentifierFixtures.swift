import Foundation
import RuntimeHostAppleCore

private let testIdentifierLock = NSLock()
@MainActor
private var nextTestSessionRawValue: UInt64 = 100
@MainActor
private var nextTestEmbedderRawValue: UInt64 = 1000

/// Create one stable test session handle.
@MainActor
func makeTestSessionHandle() -> HostSessionHandle {
    testIdentifierLock.lock()
    defer { testIdentifierLock.unlock() }

    let sessionHandle = HostSessionHandle(rawValue: nextTestSessionRawValue)
    nextTestSessionRawValue += 1

    return sessionHandle
}

/// Create one stable test embedder identifier.
@MainActor
func makeTestEmbedderID() -> HostEmbedderID {
    testIdentifierLock.lock()
    defer { testIdentifierLock.unlock() }

    let embedderID = HostEmbedderID(rawValue: nextTestEmbedderRawValue)
    nextTestEmbedderRawValue += 1

    return embedderID
}
