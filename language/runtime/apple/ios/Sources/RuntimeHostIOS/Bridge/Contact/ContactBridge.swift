import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

func contactListCallback(
  sessionHandle: UInt64,
  query: DestackRustContactQuery,
  outputPage: UnsafeMutablePointer<DestackRustContactPage>?
) -> UInt32 {
  handleContactList(
    sessionHandle: sessionHandle,
    query: query,
    outputPage: outputPage
  )
}

func contactSearchCallback(
  sessionHandle: UInt64,
  queryText: DestackRustStringRef,
  query: DestackRustContactQuery,
  outputPage: UnsafeMutablePointer<DestackRustContactPage>?
) -> UInt32 {
  handleContactSearch(
    sessionHandle: sessionHandle,
    queryText: queryText,
    query: query,
    outputPage: outputPage
  )
}

func contactReadCallback(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  outputContact: UnsafeMutablePointer<DestackRustContact>?
) -> UInt32 {
  handleContactRead(
    sessionHandle: sessionHandle,
    identifier: identifier,
    outputContact: outputContact
  )
}

func contactCreateCallback(
  sessionHandle: UInt64,
  draft: DestackRustContactDraft,
  outputIdentifier: UnsafeMutablePointer<DestackRustStringRef>?
) -> UInt32 {
  handleContactCreate(
    sessionHandle: sessionHandle,
    draft: draft,
    outputIdentifier: outputIdentifier
  )
}

func contactUpdateCallback(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  draft: DestackRustContactDraft
) -> UInt32 {
  handleContactUpdate(
    sessionHandle: sessionHandle,
    identifier: identifier,
    draft: draft
  )
}

func contactDeleteCallback(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef
) -> UInt32 {
  handleContactDelete(
    sessionHandle: sessionHandle,
    identifier: identifier
  )
}

/// One temporary native allocation arena for one contact bridge callback.
private final class ContactBridgeArena {
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

/// Return the thread-local contact bridge arena for one callback thread.
private func currentContactBridgeArena() -> ContactBridgeArena {
  let dictionary = Thread.current.threadDictionary
  let key = "dev.destack.runtime.apple.contact-bridge-arena"

  if let arena = dictionary[key] as? ContactBridgeArena {
    arena.reset()

    return arena
  }

  let arena = ContactBridgeArena()
  dictionary[key] = arena

  return arena
}

/// One contact bridge lane for one attached iOS runtime host.
@MainActor
final class ContactBridge {
  /// List one page of contacts through the attached runtime host.
  func listContacts(
    runtimeHost: RuntimeHost,
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    runtimeHost.contactRequests.listContacts(query)
  }

  /// Search contacts through the attached runtime host.
  func searchContacts(
    runtimeHost: RuntimeHost,
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    runtimeHost.contactRequests.searchContacts(queryText, query: query)
  }

  /// Read one contact through the attached runtime host.
  func readContact(
    runtimeHost: RuntimeHost,
    id: String
  ) -> RuntimeHostContactResponse {
    runtimeHost.contactRequests.readContact(id: id)
  }

  /// Create one contact through the attached runtime host.
  func createContact(
    runtimeHost: RuntimeHost,
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    runtimeHost.contactRequests.createContact(draft)
  }

  /// Update one contact through the attached runtime host.
  func updateContact(
    runtimeHost: RuntimeHost,
    id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    runtimeHost.contactRequests.updateContact(id: id, draft: draft)
  }

  /// Delete one contact through the attached runtime host.
  func deleteContact(
    runtimeHost: RuntimeHost,
    id: String
  ) -> UInt32 {
    runtimeHost.contactRequests.deleteContact(id: id)
  }
}

/// Resolve one registered contact bridge for one runtime session.
func guardContactBridge(
  sessionHandle: UInt64
) -> ContactBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.contactBridge
}

/// Handle one runtime callback asking to list contacts.
private func handleContactList(
  sessionHandle: UInt64,
  query: DestackRustContactQuery,
  outputPage: UnsafeMutablePointer<DestackRustContactPage>?
) -> UInt32 {
  guard let bridge = guardContactBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputPage,
    let query = decodeContactQuery(query)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.listContacts(
      runtimeHost: runtimeHost,
      query
    )
  }

  guard let page = response.page else {
    return response.status
  }

  return withEncodedContactPage(page) { encodedPage in
    outputPage.pointee = encodedPage

    return response.status
  }
}

/// Handle one runtime callback asking to search contacts.
private func handleContactSearch(
  sessionHandle: UInt64,
  queryText: DestackRustStringRef,
  query: DestackRustContactQuery,
  outputPage: UnsafeMutablePointer<DestackRustContactPage>?
) -> UInt32 {
  guard let bridge = guardContactBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputPage,
    let queryText = decodeContactString(queryText),
    let query = decodeContactQuery(query)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.searchContacts(
      runtimeHost: runtimeHost,
      queryText,
      query: query
    )
  }

  guard let page = response.page else {
    return response.status
  }

  return withEncodedContactPage(page) { encodedPage in
    outputPage.pointee = encodedPage

    return response.status
  }
}

/// Handle one runtime callback asking to read one contact.
private func handleContactRead(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  outputContact: UnsafeMutablePointer<DestackRustContact>?
) -> UInt32 {
  guard let bridge = guardContactBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputContact,
    let identifier = decodeContactString(identifier)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.readContact(
      runtimeHost: runtimeHost,
      id: identifier
    )
  }

  guard let contact = response.contact else {
    return response.status
  }

  return withEncodedContact(contact) { encodedContact in
    outputContact.pointee = encodedContact

    return response.status
  }
}

/// Handle one runtime callback asking to create one contact.
private func handleContactCreate(
  sessionHandle: UInt64,
  draft: DestackRustContactDraft,
  outputIdentifier: UnsafeMutablePointer<DestackRustStringRef>?
) -> UInt32 {
  guard let bridge = guardContactBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputIdentifier,
    let draft = decodeContactDraft(draft)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.createContact(
      runtimeHost: runtimeHost,
      draft
    )
  }

  guard let identifier = response.id else {
    return response.status
  }

  return withEncodedContactString(identifier) { encodedIdentifier in
    outputIdentifier.pointee = encodedIdentifier

    return response.status
  }
}

/// Handle one runtime callback asking to update one contact.
private func handleContactUpdate(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  draft: DestackRustContactDraft
) -> UInt32 {
  guard let bridge = guardContactBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let identifier = decodeContactString(identifier),
    let draft = decodeContactDraft(draft)
  else {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.updateContact(
      runtimeHost: runtimeHost,
      id: identifier,
      draft: draft
    )
  }
}

/// Handle one runtime callback asking to delete one contact.
private func handleContactDelete(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardContactBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let identifier = decodeContactString(identifier) else {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.deleteContact(
      runtimeHost: runtimeHost,
      id: identifier
    )
  }
}

/// Decode one native string reference into one Swift string.
private func decodeContactString(
  _ value: DestackRustStringRef
) -> String? {
  guard let data = value.data else {
    return value.len == 0 ? "" : nil
  }

  let bytes = UnsafeBufferPointer(start: data, count: Int(value.len))

  return String(bytes: bytes, encoding: .utf8)
}

/// Decode one native contact query into one Swift payload.
private func decodeContactQuery(
  _ value: DestackRustContactQuery
) -> RuntimeHostContactQuery? {
  let cursor: String?

  // cursor
  if value.has_cursor {
    guard let decodedCursor = decodeContactString(value.cursor) else {
      return nil
    }

    cursor = decodedCursor
  } else {
    cursor = nil
  }

  // limit
  let limit = value.has_limit ? value.limit : nil

  return RuntimeHostContactQuery(
    cursor: cursor,
    limit: limit,
    includePhones: value.include_phones,
    includeEmails: value.include_emails,
    includeAddresses: value.include_addresses,
    includeOrganization: value.include_organization,
    includeNotes: value.include_notes
  )
}

/// Decode one native contact-name payload into one Swift value.
private func decodeContactName(
  _ value: DestackRustContactName
) -> RuntimeHostContactName? {
  guard
    let givenName = decodeContactString(value.given_name),
    let middleName = decodeContactString(value.middle_name),
    let familyName = decodeContactString(value.family_name),
    let prefix = decodeContactString(value.prefix),
    let suffix = decodeContactString(value.suffix),
    let nickname = decodeContactString(value.nickname),
    let phoneticGivenName = decodeContactString(value.phonetic_given_name),
    let phoneticFamilyName = decodeContactString(value.phonetic_family_name)
  else {
    return nil
  }

  return RuntimeHostContactName(
    givenName: givenName,
    middleName: middleName,
    familyName: familyName,
    prefix: prefix,
    suffix: suffix,
    nickname: nickname,
    phoneticGivenName: phoneticGivenName,
    phoneticFamilyName: phoneticFamilyName
  )
}

/// Decode one native phone slice into one Swift array.
private func decodeContactPhones(
  _ values: DestackRustContactPhoneSlice
) -> [RuntimeHostContactPhone]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return buffer.compactMap { value in
    guard
      let label = decodeContactString(value.label),
      let number = decodeContactString(value.number),
      let normalizedNumber = decodeContactString(value.normalized_number)
    else {
      return nil
    }

    return RuntimeHostContactPhone(
      label: label,
      number: number,
      normalizedNumber: normalizedNumber,
      primary: value.primary
    )
  }
}

/// Decode one native email slice into one Swift array.
private func decodeContactEmails(
  _ values: DestackRustContactEmailSlice
) -> [RuntimeHostContactEmail]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return buffer.compactMap { value in
    guard
      let label = decodeContactString(value.label),
      let address = decodeContactString(value.address)
    else {
      return nil
    }

    return RuntimeHostContactEmail(
      label: label,
      address: address,
      primary: value.primary
    )
  }
}

/// Decode one native address slice into one Swift array.
private func decodeContactAddresses(
  _ values: DestackRustContactAddressSlice
) -> [RuntimeHostContactAddress]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return buffer.compactMap { value in
    guard
      let label = decodeContactString(value.label),
      let street = decodeContactString(value.street),
      let city = decodeContactString(value.city),
      let region = decodeContactString(value.region),
      let postalCode = decodeContactString(value.postal_code),
      let country = decodeContactString(value.country),
      let countryCode = decodeContactString(value.country_code)
    else {
      return nil
    }

    return RuntimeHostContactAddress(
      label: label,
      street: street,
      city: city,
      region: region,
      postalCode: postalCode,
      country: country,
      countryCode: countryCode
    )
  }
}

/// Decode one native contact organization into one Swift value.
private func decodeContactOrganization(
  _ value: DestackRustContactOrganization
) -> RuntimeHostContactOrganization? {
  guard
    let company = decodeContactString(value.company),
    let department = decodeContactString(value.department),
    let title = decodeContactString(value.title)
  else {
    return nil
  }

  return RuntimeHostContactOrganization(
    company: company,
    department: department,
    title: title
  )
}

/// Decode one native contact draft into one Swift value.
private func decodeContactDraft(
  _ value: DestackRustContactDraft
) -> RuntimeHostContactDraft? {
  guard
    let name = decodeContactName(value.name),
    let phones = decodeContactPhones(value.phones),
    let emails = decodeContactEmails(value.emails),
    let addresses = decodeContactAddresses(value.addresses),
    let organization = decodeContactOrganization(value.organization),
    let note = decodeContactString(value.note)
  else {
    return nil
  }

  return RuntimeHostContactDraft(
    name: name,
    phones: phones,
    emails: emails,
    addresses: addresses,
    organization: organization,
    note: note
  )
}

/// Execute one body with one encoded native string reference.
private func withEncodedContactString<T>(
  _ value: String,
  body: (DestackRustStringRef) -> T
) -> T {
  let arena = currentContactBridgeArena()
  let encodedValue = arena.makeStringRef(value)

  return body(encodedValue)
}

/// Execute one body with one encoded native contact payload.
private func withEncodedContact<T>(
  _ contact: RuntimeHostContact,
  body: (DestackRustContact) -> T
) -> T {
  let arena = currentContactBridgeArena()
  let encodedContact = encodeContact(contact, arena: arena)

  return body(encodedContact)
}

/// Execute one body with one encoded native contact page payload.
private func withEncodedContactPage<T>(
  _ page: RuntimeHostContactPage,
  body: (DestackRustContactPage) -> T
) -> T {
  let arena = currentContactBridgeArena()
  let encodedPage = encodeContactPage(page, arena: arena)

  return body(encodedPage)
}

/// Encode one Swift contact-name payload into one native value.
private func encodeContactName(
  _ value: RuntimeHostContactName,
  arena: ContactBridgeArena
) -> DestackRustContactName {
  DestackRustContactName(
    given_name: arena.makeStringRef(value.givenName),
    middle_name: arena.makeStringRef(value.middleName),
    family_name: arena.makeStringRef(value.familyName),
    prefix: arena.makeStringRef(value.prefix),
    suffix: arena.makeStringRef(value.suffix),
    nickname: arena.makeStringRef(value.nickname),
    phonetic_given_name: arena.makeStringRef(value.phoneticGivenName),
    phonetic_family_name: arena.makeStringRef(value.phoneticFamilyName)
  )
}

/// Encode one Swift phone slice into one native slice.
private func encodeContactPhones(
  _ values: [RuntimeHostContactPhone],
  arena: ContactBridgeArena
) -> DestackRustContactPhoneSlice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = DestackRustContactPhone(
        label: arena.makeStringRef(value.label),
        number: arena.makeStringRef(value.number),
        normalized_number: arena.makeStringRef(value.normalizedNumber),
        primary: value.primary
      )
    }
  }

  return DestackRustContactPhoneSlice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one Swift email slice into one native slice.
private func encodeContactEmails(
  _ values: [RuntimeHostContactEmail],
  arena: ContactBridgeArena
) -> DestackRustContactEmailSlice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = DestackRustContactEmail(
        label: arena.makeStringRef(value.label),
        address: arena.makeStringRef(value.address),
        primary: value.primary
      )
    }
  }

  return DestackRustContactEmailSlice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one Swift address slice into one native slice.
private func encodeContactAddresses(
  _ values: [RuntimeHostContactAddress],
  arena: ContactBridgeArena
) -> DestackRustContactAddressSlice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = DestackRustContactAddress(
        label: arena.makeStringRef(value.label),
        street: arena.makeStringRef(value.street),
        city: arena.makeStringRef(value.city),
        region: arena.makeStringRef(value.region),
        postal_code: arena.makeStringRef(value.postalCode),
        country: arena.makeStringRef(value.country),
        country_code: arena.makeStringRef(value.countryCode)
      )
    }
  }

  return DestackRustContactAddressSlice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one Swift organization payload into one native value.
private func encodeContactOrganization(
  _ value: RuntimeHostContactOrganization,
  arena: ContactBridgeArena
) -> DestackRustContactOrganization {
  DestackRustContactOrganization(
    company: arena.makeStringRef(value.company),
    department: arena.makeStringRef(value.department),
    title: arena.makeStringRef(value.title)
  )
}

/// Encode one Swift contact payload into one native value.
private func encodeContact(
  _ value: RuntimeHostContact,
  arena: ContactBridgeArena
) -> DestackRustContact {
  DestackRustContact(
    id: arena.makeStringRef(value.id),
    name: encodeContactName(value.name, arena: arena),
    phones: encodeContactPhones(value.phones, arena: arena),
    emails: encodeContactEmails(value.emails, arena: arena),
    addresses: encodeContactAddresses(value.addresses, arena: arena),
    organization: encodeContactOrganization(value.organization, arena: arena),
    note: arena.makeStringRef(value.note)
  )
}

/// Encode one Swift contact page payload into one native value.
private func encodeContactPage(
  _ value: RuntimeHostContactPage,
  arena: ContactBridgeArena
) -> DestackRustContactPage {
  let contacts = arena.makeArray(count: value.contacts.count) { buffer in
    for (index, contact) in value.contacts.enumerated() {
      buffer[index] = encodeContact(contact, arena: arena)
    }
  }

  return DestackRustContactPage(
    contacts: DestackRustContactSlice(
      data: contacts,
      len: UInt32(value.contacts.count)
    ),
    next_cursor: arena.makeStringRef(value.nextCursor),
    has_more: value.hasMore
  )
}
