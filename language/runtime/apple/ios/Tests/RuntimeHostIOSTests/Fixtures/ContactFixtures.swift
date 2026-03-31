import RuntimeHostAppleCore

/// One no-op contact request handler for iOS tests.
@MainActor
final class IOSNoopContactRequestHandler: ContactRequests {
  func list(
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    RuntimeHostContactPageResponse(status: hostStatusNotSupported)
  }

  func search(
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    RuntimeHostContactPageResponse(status: hostStatusNotSupported)
  }

  func read(
    _ id: String
  ) -> RuntimeHostContactResponse {
    RuntimeHostContactResponse(status: hostStatusNotSupported)
  }

  func create(
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    RuntimeHostContactCreateResponse(status: hostStatusNotSupported)
  }

  func update(
    _ id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func deleteContact(
    _ id: String
  ) -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording contact request handler for iOS tests.
@MainActor
final class IOSRecordingContactRequestHandler: ContactRequests {
  var listQueries: [RuntimeHostContactQuery] = []

  func list(
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    listQueries.append(query)

    return RuntimeHostContactPageResponse(status: hostStatusOk)
  }

  func search(
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    RuntimeHostContactPageResponse(status: hostStatusOk)
  }

  func read(
    _ id: String
  ) -> RuntimeHostContactResponse {
    RuntimeHostContactResponse(status: hostStatusOk)
  }

  func create(
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    RuntimeHostContactCreateResponse(status: hostStatusOk)
  }

  func update(
    _ id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    hostStatusOk
  }

  func deleteContact(
    _ id: String
  ) -> UInt32 {
    hostStatusOk
  }
}
