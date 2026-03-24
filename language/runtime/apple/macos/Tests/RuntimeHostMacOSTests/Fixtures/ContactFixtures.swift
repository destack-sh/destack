import RuntimeHostAppleCore

/// One no-op contact request handler for macOS tests.
@MainActor
final class MacOSNoopContactRequestHandler: ContactRequests {
  func listContacts(
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    RuntimeHostContactPageResponse(status: hostStatusNotSupported)
  }

  func searchContacts(
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    RuntimeHostContactPageResponse(status: hostStatusNotSupported)
  }

  func readContact(
    id: String
  ) -> RuntimeHostContactResponse {
    RuntimeHostContactResponse(status: hostStatusNotSupported)
  }

  func createContact(
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    RuntimeHostContactCreateResponse(status: hostStatusNotSupported)
  }

  func updateContact(
    id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func deleteContact(
    id: String
  ) -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording contact request handler for macOS tests.
@MainActor
final class MacOSRecordingContactRequestHandler: ContactRequests {
  var listQueries: [RuntimeHostContactQuery] = []

  func listContacts(
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    listQueries.append(query)

    return RuntimeHostContactPageResponse(status: hostStatusOk)
  }

  func searchContacts(
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    RuntimeHostContactPageResponse(status: hostStatusOk)
  }

  func readContact(
    id: String
  ) -> RuntimeHostContactResponse {
    RuntimeHostContactResponse(status: hostStatusOk)
  }

  func createContact(
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    RuntimeHostContactCreateResponse(status: hostStatusOk)
  }

  func updateContact(
    id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    hostStatusOk
  }

  func deleteContact(
    id: String
  ) -> UInt32 {
    hostStatusOk
  }
}
