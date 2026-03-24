import Foundation

/// The Apple contact request surface for one runtime host embedder.
@MainActor
public protocol ContactRequests: AnyObject {
    /// List one page of contacts for one query.
    func listContacts(
        _ query: RuntimeHostContactQuery
    ) -> RuntimeHostContactPageResponse

    /// Search contacts for one query string and one query shape.
    func searchContacts(
        _ queryText: String,
        query: RuntimeHostContactQuery
    ) -> RuntimeHostContactPageResponse

    /// Read one contact by stable identifier.
    func readContact(
        id: String
    ) -> RuntimeHostContactResponse

    /// Create one contact and return its stable identifier.
    func createContact(
        _ draft: RuntimeHostContactDraft
    ) -> RuntimeHostContactCreateResponse

    /// Update one contact by stable identifier.
    func updateContact(
        id: String,
        draft: RuntimeHostContactDraft
    ) -> UInt32

    /// Delete one contact by stable identifier.
    func deleteContact(
        id: String
    ) -> UInt32
}

/// The explicit unsupported contact request surface for one Apple runtime host.
@MainActor
public final class UnsupportedContactRequests: ContactRequests {
    /// Create one unsupported contact request surface.
    public init() {}

    public func listContacts(
        _ query: RuntimeHostContactQuery
    ) -> RuntimeHostContactPageResponse {
        RuntimeHostContactPageResponse(status: hostStatusNotSupported)
    }

    public func searchContacts(
        _ queryText: String,
        query: RuntimeHostContactQuery
    ) -> RuntimeHostContactPageResponse {
        RuntimeHostContactPageResponse(status: hostStatusNotSupported)
    }

    public func readContact(
        id: String
    ) -> RuntimeHostContactResponse {
        RuntimeHostContactResponse(status: hostStatusNotSupported)
    }

    public func createContact(
        _ draft: RuntimeHostContactDraft
    ) -> RuntimeHostContactCreateResponse {
        RuntimeHostContactCreateResponse(status: hostStatusNotSupported)
    }

    public func updateContact(
        id: String,
        draft: RuntimeHostContactDraft
    ) -> UInt32 {
        hostStatusNotSupported
    }

    public func deleteContact(
        id: String
    ) -> UInt32 {
        hostStatusNotSupported
    }
}
