import Contacts
import Foundation

/// The Apple contact request surface backed by `CNContactStore`.
@MainActor
public final class ContactStoreRequests: ContactRequests {
  /// The underlying Apple contact store.
  private let contactStore: CNContactStore

  /// Create one Apple contact request surface.
  public init(
    contactStore: CNContactStore = CNContactStore()
  ) {
    self.contactStore = contactStore
  }

  public func listContacts(
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    performListContacts(
      contactStore: contactStore,
      query: query
    )
  }

  public func searchContacts(
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    performSearchContacts(
      contactStore: contactStore,
      queryText: queryText,
      query: query
    )
  }

  public func readContact(
    id: String
  ) -> RuntimeHostContactResponse {
    performReadContact(
      contactStore: contactStore,
      id: id
    )
  }

  public func createContact(
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    performCreateContact(
      contactStore: contactStore,
      draft: draft
    )
  }

  public func updateContact(
    id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    performUpdateContact(
      contactStore: contactStore,
      id: id,
      draft: draft
    )
  }

  public func deleteContact(
    id: String
  ) -> UInt32 {
    performDeleteContact(
      contactStore: contactStore,
      id: id
    )
  }
}

/// One decoded contact page window.
private struct ContactPageWindow {
  /// The stable result offset.
  let offset: Int

  /// The decoded page size.
  let limit: Int
}

/// List one page of contacts through one Apple contact store.
@MainActor
private func performListContacts(
  contactStore: CNContactStore,
  query: RuntimeHostContactQuery
) -> RuntimeHostContactPageResponse {
  // authorization
  guard isContactAccessAuthorized() else {
    return RuntimeHostContactPageResponse(status: hostStatusPermissionDenied)
  }

  // paging
  guard let pageWindow = decodeContactPageWindow(query) else {
    return RuntimeHostContactPageResponse(status: hostStatusInvalidArgument)
  }

  // fetch
  let keys = contactKeys(query)
  let request = CNContactFetchRequest(keysToFetch: keys)
  request.unifyResults = true
  request.sortOrder = .userDefault

  do {
    var contacts: [RuntimeHostContact] = []

    try contactStore.enumerateContacts(with: request) { contact, _ in
      contacts.append(runtimeHostContact(contact, query: query))
    }

    let page = paginateContacts(
      contacts,
      pageWindow: pageWindow
    )

    return RuntimeHostContactPageResponse(
      status: hostStatusOk,
      page: page
    )
  } catch {
    return RuntimeHostContactPageResponse(status: hostStatusFailed)
  }
}

/// Search contacts through one Apple contact store.
@MainActor
private func performSearchContacts(
  contactStore: CNContactStore,
  queryText: String,
  query: RuntimeHostContactQuery
) -> RuntimeHostContactPageResponse {
  // authorization
  guard isContactAccessAuthorized() else {
    return RuntimeHostContactPageResponse(status: hostStatusPermissionDenied)
  }

  // paging
  guard let pageWindow = decodeContactPageWindow(query) else {
    return RuntimeHostContactPageResponse(status: hostStatusInvalidArgument)
  }

  // empty search
  if queryText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
    return performListContacts(
      contactStore: contactStore,
      query: query
    )
  }

  // native search indexes
  let keys = contactKeys(query)
  var contactsByID: [String: RuntimeHostContact] = [:]
  appendSearchContacts(
    into: &contactsByID,
    contacts: searchContactsByName(
      contactStore: contactStore,
      queryText: queryText,
      keys: keys
    ),
    query: query
  )

  if queryText.contains("@") {
    appendSearchContacts(
      into: &contactsByID,
      contacts: searchContactsByEmail(
        contactStore: contactStore,
        queryText: queryText,
        keys: keys
      ),
      query: query
    )
  }

  if queryText.contains(where: \.isNumber) {
    appendSearchContacts(
      into: &contactsByID,
      contacts: searchContactsByPhone(
        contactStore: contactStore,
        queryText: queryText,
        keys: keys
      ),
      query: query
    )
  }

  let contacts = contactsByID.values.sorted { left, right in
    contactSortKey(left) < contactSortKey(right)
  }
  let page = paginateContacts(
    contacts,
    pageWindow: pageWindow
  )

  return RuntimeHostContactPageResponse(
    status: hostStatusOk,
    page: page
  )
}

/// Read one contact by stable identifier through one Apple contact store.
@MainActor
private func performReadContact(
  contactStore: CNContactStore,
  id: String
) -> RuntimeHostContactResponse {
  // authorization
  guard isContactAccessAuthorized() else {
    return RuntimeHostContactResponse(status: hostStatusPermissionDenied)
  }

  // lookup
  guard
    let contact = try? contactStore.unifiedContact(
      withIdentifier: id,
      keysToFetch: contactKeys(fullContactQuery())
    )
  else {
    return RuntimeHostContactResponse(status: hostStatusNotFound)
  }

  return RuntimeHostContactResponse(
    status: hostStatusOk,
    contact: runtimeHostContact(contact, query: fullContactQuery())
  )
}

/// Create one contact through one Apple contact store.
@MainActor
private func performCreateContact(
  contactStore: CNContactStore,
  draft: RuntimeHostContactDraft
) -> RuntimeHostContactCreateResponse {
  // authorization
  guard isContactAccessAuthorized() else {
    return RuntimeHostContactCreateResponse(status: hostStatusPermissionDenied)
  }

  // save
  let contact = CNMutableContact()
  applyContactDraft(
    draft,
    to: contact
  )
  let saveRequest = CNSaveRequest()
  saveRequest.add(contact, toContainerWithIdentifier: nil)

  do {
    try contactStore.execute(saveRequest)

    return RuntimeHostContactCreateResponse(
      status: hostStatusOk,
      id: contact.identifier
    )
  } catch {
    return RuntimeHostContactCreateResponse(status: hostStatusFailed)
  }
}

/// Update one contact through one Apple contact store.
@MainActor
private func performUpdateContact(
  contactStore: CNContactStore,
  id: String,
  draft: RuntimeHostContactDraft
) -> UInt32 {
  // authorization
  guard isContactAccessAuthorized() else {
    return hostStatusPermissionDenied
  }

  // lookup
  guard
    let unifiedContact = try? contactStore.unifiedContact(
      withIdentifier: id,
      keysToFetch: contactKeys(fullContactQuery())
    )
  else {
    return hostStatusNotFound
  }

  do {
    let contact = unifiedContact.mutableCopy() as! CNMutableContact
    applyContactDraft(
      draft,
      to: contact
    )
    let saveRequest = CNSaveRequest()
    saveRequest.update(contact)
    try contactStore.execute(saveRequest)

    return hostStatusOk
  } catch {
    return hostStatusFailed
  }
}

/// Delete one contact through one Apple contact store.
@MainActor
private func performDeleteContact(
  contactStore: CNContactStore,
  id: String
) -> UInt32 {
  // authorization
  guard isContactAccessAuthorized() else {
    return hostStatusPermissionDenied
  }

  // lookup
  guard
    let unifiedContact = try? contactStore.unifiedContact(
      withIdentifier: id,
      keysToFetch: contactKeys(RuntimeHostContactQuery())
    )
  else {
    return hostStatusNotFound
  }

  do {
    let contact = unifiedContact.mutableCopy() as! CNMutableContact
    let saveRequest = CNSaveRequest()
    saveRequest.delete(contact)
    try contactStore.execute(saveRequest)

    return hostStatusOk
  } catch {
    return hostStatusFailed
  }
}

/// Return whether the Apple contact store is authorized for the current process.
@MainActor
private func isContactAccessAuthorized() -> Bool {
  return CNContactStore.authorizationStatus(for: .contacts) == .authorized
}

/// Decode one stable contact page window.
private func decodeContactPageWindow(
  _ query: RuntimeHostContactQuery
) -> ContactPageWindow? {
  // cursor
  let offset: Int
  if let cursor = query.cursor {
    guard let decodedOffset = Int(cursor), decodedOffset >= 0 else {
      return nil
    }

    offset = decodedOffset
  } else {
    offset = 0
  }

  // limit
  let limit = Int(query.limit ?? UInt32(runtimeHostContactDefaultPageLimit))
  guard limit > 0 else {
    return nil
  }

  return ContactPageWindow(
    offset: offset,
    limit: limit
  )
}

/// Build the full runtime contact query payload.
private func fullContactQuery() -> RuntimeHostContactQuery {
  RuntimeHostContactQuery(
    includePhones: true,
    includeEmails: true,
    includeAddresses: true,
    includeOrganization: true,
    includeNotes: true
  )
}

/// Build one native key list for one runtime query.
private func contactKeys(
  _ query: RuntimeHostContactQuery
) -> [CNKeyDescriptor] {
  var keys: [CNKeyDescriptor] = [
    CNContactIdentifierKey as CNKeyDescriptor,
    CNContactGivenNameKey as CNKeyDescriptor,
    CNContactMiddleNameKey as CNKeyDescriptor,
    CNContactFamilyNameKey as CNKeyDescriptor,
    CNContactNamePrefixKey as CNKeyDescriptor,
    CNContactNameSuffixKey as CNKeyDescriptor,
    CNContactNicknameKey as CNKeyDescriptor,
    CNContactPhoneticGivenNameKey as CNKeyDescriptor,
    CNContactPhoneticFamilyNameKey as CNKeyDescriptor,
  ]

  if query.includePhones {
    keys.append(CNContactPhoneNumbersKey as CNKeyDescriptor)
  }

  if query.includeEmails {
    keys.append(CNContactEmailAddressesKey as CNKeyDescriptor)
  }

  if query.includeAddresses {
    keys.append(CNContactPostalAddressesKey as CNKeyDescriptor)
  }

  if query.includeOrganization {
    keys.append(CNContactOrganizationNameKey as CNKeyDescriptor)
    keys.append(CNContactDepartmentNameKey as CNKeyDescriptor)
    keys.append(CNContactJobTitleKey as CNKeyDescriptor)
  }

  if query.includeNotes {
    keys.append(CNContactNoteKey as CNKeyDescriptor)
  }

  return keys
}

/// Search contacts by displayable name.
@MainActor
private func searchContactsByName(
  contactStore: CNContactStore,
  queryText: String,
  keys: [CNKeyDescriptor]
) -> [CNContact] {
  do {
    return try contactStore.unifiedContacts(
      matching: CNContact.predicateForContacts(matchingName: queryText),
      keysToFetch: keys
    )
  } catch {
    return []
  }
}

/// Search contacts by email address.
@MainActor
private func searchContactsByEmail(
  contactStore: CNContactStore,
  queryText: String,
  keys: [CNKeyDescriptor]
) -> [CNContact] {
  do {
    return try contactStore.unifiedContacts(
      matching: CNContact.predicateForContacts(matchingEmailAddress: queryText),
      keysToFetch: keys
    )
  } catch {
    return []
  }
}

/// Search contacts by phone number.
@MainActor
private func searchContactsByPhone(
  contactStore: CNContactStore,
  queryText: String,
  keys: [CNKeyDescriptor]
) -> [CNContact] {
  do {
    return try contactStore.unifiedContacts(
      matching: CNContact.predicateForContacts(
        matching: CNPhoneNumber(stringValue: queryText)
      ),
      keysToFetch: keys
    )
  } catch {
    return []
  }
}

/// Append one native contact sequence into one deduplicated runtime map.
private func appendSearchContacts(
  into results: inout [String: RuntimeHostContact],
  contacts: [CNContact],
  query: RuntimeHostContactQuery
) {
  for contact in contacts {
    let runtimeContact = runtimeHostContact(
      contact,
      query: query
    )

    results[runtimeContact.id] = runtimeContact
  }
}

/// Page one sorted contact result list.
private func paginateContacts(
  _ contacts: [RuntimeHostContact],
  pageWindow: ContactPageWindow
) -> RuntimeHostContactPage {
  let pageContacts =
    contacts
    .dropFirst(pageWindow.offset)
    .prefix(pageWindow.limit + 1)
  let hasMore = pageContacts.count > pageWindow.limit
  let visibleContacts = hasMore ? Array(pageContacts.dropLast()) : Array(pageContacts)
  let nextCursor = hasMore ? String(pageWindow.offset + visibleContacts.count) : ""

  return RuntimeHostContactPage(
    contacts: visibleContacts,
    nextCursor: nextCursor,
    hasMore: hasMore
  )
}

/// Materialize one runtime contact payload from one native contact.
private func runtimeHostContact(
  _ contact: CNContact,
  query: RuntimeHostContactQuery
) -> RuntimeHostContact {
  let phones =
    query.includePhones
    ? contact.phoneNumbers.enumerated().map { index, value in
      RuntimeHostContactPhone(
        label: localizedLabel(value.label),
        number: value.value.stringValue,
        normalizedNumber: value.value.stringValue,
        primary: index == 0
      )
    } : []
  let emails =
    query.includeEmails
    ? contact.emailAddresses.enumerated().map { index, value in
      RuntimeHostContactEmail(
        label: localizedLabel(value.label),
        address: value.value as String,
        primary: index == 0
      )
    } : []
  let addresses: [RuntimeHostContactAddress] =
    query.includeAddresses
    ? contact.postalAddresses.map { value in
      let address = value.value

      return RuntimeHostContactAddress(
        label: localizedLabel(value.label),
        street: address.street,
        city: address.city,
        region: address.state,
        postalCode: address.postalCode,
        country: address.country,
        countryCode: address.isoCountryCode
      )
    } : []
  let organization =
    query.includeOrganization
    ? RuntimeHostContactOrganization(
      company: contact.organizationName,
      department: contact.departmentName,
      title: contact.jobTitle
    )
    : RuntimeHostContactOrganization()
  let note = query.includeNotes ? contact.note : ""

  return RuntimeHostContact(
    id: contact.identifier,
    name: RuntimeHostContactName(
      givenName: contact.givenName,
      middleName: contact.middleName,
      familyName: contact.familyName,
      prefix: contact.namePrefix,
      suffix: contact.nameSuffix,
      nickname: contact.nickname,
      phoneticGivenName: contact.phoneticGivenName,
      phoneticFamilyName: contact.phoneticFamilyName
    ),
    phones: phones,
    emails: emails,
    addresses: addresses,
    organization: organization,
    note: note
  )
}

/// Apply one runtime draft onto one mutable native contact.
private func applyContactDraft(
  _ draft: RuntimeHostContactDraft,
  to contact: CNMutableContact
) {
  // scalar fields
  contact.givenName = draft.name.givenName
  contact.middleName = draft.name.middleName
  contact.familyName = draft.name.familyName
  contact.namePrefix = draft.name.prefix
  contact.nameSuffix = draft.name.suffix
  contact.nickname = draft.name.nickname
  contact.phoneticGivenName = draft.name.phoneticGivenName
  contact.phoneticFamilyName = draft.name.phoneticFamilyName
  contact.organizationName = draft.organization.company
  contact.departmentName = draft.organization.department
  contact.jobTitle = draft.organization.title
  contact.note = draft.note

  // phone rows
  let phones = draft.phones
    .sorted { left, right in
      if left.primary != right.primary {
        return left.primary && !right.primary
      }

      return left.label < right.label
    }
    .map { phone in
      CNLabeledValue(
        label: contactLabel(phone.label),
        value: CNPhoneNumber(stringValue: phone.number)
      )
    }
  contact.phoneNumbers = phones

  // email rows
  let emails = draft.emails
    .sorted { left, right in
      if left.primary != right.primary {
        return left.primary && !right.primary
      }

      return left.label < right.label
    }
    .map { email in
      CNLabeledValue(
        label: contactLabel(email.label),
        value: email.address as NSString
      )
    }
  contact.emailAddresses = emails

  // address rows
  let addresses = draft.addresses.map { address in
    let value = CNMutablePostalAddress()
    value.street = address.street
    value.city = address.city
    value.state = address.region
    value.postalCode = address.postalCode
    value.country = address.country
    value.isoCountryCode = address.countryCode

    return CNLabeledValue(
      label: contactLabel(address.label),
      value: value.copy() as! CNPostalAddress
    )
  }
  contact.postalAddresses = addresses
}

/// Return one localized user-visible label for one native contact label.
private func localizedLabel(
  _ label: String?
) -> String {
  guard let label else {
    return ""
  }

  return CNLabeledValue<NSString>.localizedString(forLabel: label)
}

/// Map one runtime label back into one native Apple contact label.
private func contactLabel(
  _ label: String
) -> String {
  switch label.lowercased() {
  case "home":
    return CNLabelHome
  case "work":
    return CNLabelWork
  case "other":
    return CNLabelOther
  case "mobile":
    return CNLabelPhoneNumberMobile
  case "main":
    return CNLabelPhoneNumberMain
  case "fax home":
    return CNLabelPhoneNumberHomeFax
  case "fax work":
    return CNLabelPhoneNumberWorkFax
  case "pager":
    return CNLabelPhoneNumberPager
  default:
    return label
  }
}

/// Build one stable runtime contact sort key.
private func contactSortKey(
  _ contact: RuntimeHostContact
) -> String {
  let visibleName = [
    contact.name.givenName,
    contact.name.middleName,
    contact.name.familyName,
    contact.organization.company,
  ]
  .filter { !$0.isEmpty }
  .joined(separator: " ")

  return "\(visibleName.lowercased()) \(contact.id)"
}
