import Foundation

/// The default contact page size when one query omits an explicit limit.
let runtimeHostContactDefaultPageLimit: Int = 50

/// One Apple contact query submitted by one runtime session.
public struct RuntimeHostContactQuery: Sendable, Hashable, Codable {
    /// The opaque cursor from one prior list or search call.
    public let cursor: String?
    /// The maximum returned contacts for this page when available.
    public let limit: UInt32?
    /// Whether phone values should be returned.
    public let includePhones: Bool
    /// Whether email values should be returned.
    public let includeEmails: Bool
    /// Whether postal-address values should be returned.
    public let includeAddresses: Bool
    /// Whether organization metadata should be returned.
    public let includeOrganization: Bool
    /// Whether note fields should be returned.
    public let includeNotes: Bool

    /// Create one contact query payload.
    public init(
        cursor: String? = nil,
        limit: UInt32? = nil,
        includePhones: Bool = false,
        includeEmails: Bool = false,
        includeAddresses: Bool = false,
        includeOrganization: Bool = false,
        includeNotes: Bool = false
    ) {
        self.cursor = cursor
        self.limit = limit
        self.includePhones = includePhones
        self.includeEmails = includeEmails
        self.includeAddresses = includeAddresses
        self.includeOrganization = includeOrganization
        self.includeNotes = includeNotes
    }
}

/// One structured Apple contact-name payload.
public struct RuntimeHostContactName: Sendable, Hashable, Codable {
    /// The given or first name.
    public let givenName: String
    /// The middle name.
    public let middleName: String
    /// The family or last name.
    public let familyName: String
    /// The honorific prefix.
    public let prefix: String
    /// The honorific suffix.
    public let suffix: String
    /// The nickname.
    public let nickname: String
    /// The phonetic given name.
    public let phoneticGivenName: String
    /// The phonetic family name.
    public let phoneticFamilyName: String

    /// Create one contact-name payload.
    public init(
        givenName: String = "",
        middleName: String = "",
        familyName: String = "",
        prefix: String = "",
        suffix: String = "",
        nickname: String = "",
        phoneticGivenName: String = "",
        phoneticFamilyName: String = ""
    ) {
        self.givenName = givenName
        self.middleName = middleName
        self.familyName = familyName
        self.prefix = prefix
        self.suffix = suffix
        self.nickname = nickname
        self.phoneticGivenName = phoneticGivenName
        self.phoneticFamilyName = phoneticFamilyName
    }
}

/// One Apple contact phone payload.
public struct RuntimeHostContactPhone: Sendable, Hashable, Codable {
    /// The user-visible label for this phone value.
    public let label: String
    /// The original phone number string.
    public let number: String
    /// The normalized phone number string when available.
    public let normalizedNumber: String
    /// Whether this phone value is marked as primary.
    public let primary: Bool

    /// Create one contact-phone payload.
    public init(
        label: String = "",
        number: String = "",
        normalizedNumber: String = "",
        primary: Bool = false
    ) {
        self.label = label
        self.number = number
        self.normalizedNumber = normalizedNumber
        self.primary = primary
    }
}

/// One Apple contact email payload.
public struct RuntimeHostContactEmail: Sendable, Hashable, Codable {
    /// The user-visible label for this email value.
    public let label: String
    /// The email address.
    public let address: String
    /// Whether this email value is marked as primary.
    public let primary: Bool

    /// Create one contact-email payload.
    public init(
        label: String = "",
        address: String = "",
        primary: Bool = false
    ) {
        self.label = label
        self.address = address
        self.primary = primary
    }
}

/// One Apple contact postal-address payload.
public struct RuntimeHostContactAddress: Sendable, Hashable, Codable {
    /// The user-visible label for this address value.
    public let label: String
    /// The street-line payload.
    public let street: String
    /// The city payload.
    public let city: String
    /// The region or state payload.
    public let region: String
    /// The postal-code payload.
    public let postalCode: String
    /// The country payload.
    public let country: String
    /// The country-code payload.
    public let countryCode: String

    /// Create one contact-address payload.
    public init(
        label: String = "",
        street: String = "",
        city: String = "",
        region: String = "",
        postalCode: String = "",
        country: String = "",
        countryCode: String = ""
    ) {
        self.label = label
        self.street = street
        self.city = city
        self.region = region
        self.postalCode = postalCode
        self.country = country
        self.countryCode = countryCode
    }
}

/// One Apple contact organization payload.
public struct RuntimeHostContactOrganization: Sendable, Hashable, Codable {
    /// The company or organization name.
    public let company: String
    /// The department name.
    public let department: String
    /// The job title.
    public let title: String

    /// Create one contact-organization payload.
    public init(
        company: String = "",
        department: String = "",
        title: String = ""
    ) {
        self.company = company
        self.department = department
        self.title = title
    }
}

/// One Apple contact payload.
public struct RuntimeHostContact: Sendable, Hashable, Codable {
    /// The stable host contact identifier.
    public let id: String
    /// The structured name payload.
    public let name: RuntimeHostContactName
    /// The phone values.
    public let phones: [RuntimeHostContactPhone]
    /// The email values.
    public let emails: [RuntimeHostContactEmail]
    /// The address values.
    public let addresses: [RuntimeHostContactAddress]
    /// The organization metadata.
    public let organization: RuntimeHostContactOrganization
    /// The contact note payload.
    public let note: String

    /// Create one contact payload.
    public init(
        id: String,
        name: RuntimeHostContactName = RuntimeHostContactName(),
        phones: [RuntimeHostContactPhone] = [],
        emails: [RuntimeHostContactEmail] = [],
        addresses: [RuntimeHostContactAddress] = [],
        organization: RuntimeHostContactOrganization = RuntimeHostContactOrganization(),
        note: String = ""
    ) {
        self.id = id
        self.name = name
        self.phones = phones
        self.emails = emails
        self.addresses = addresses
        self.organization = organization
        self.note = note
    }
}

/// One Apple contact draft payload.
public struct RuntimeHostContactDraft: Sendable, Hashable, Codable {
    /// The structured name payload.
    public let name: RuntimeHostContactName
    /// The phone values.
    public let phones: [RuntimeHostContactPhone]
    /// The email values.
    public let emails: [RuntimeHostContactEmail]
    /// The address values.
    public let addresses: [RuntimeHostContactAddress]
    /// The organization metadata.
    public let organization: RuntimeHostContactOrganization
    /// The contact note payload.
    public let note: String

    /// Create one contact-draft payload.
    public init(
        name: RuntimeHostContactName = RuntimeHostContactName(),
        phones: [RuntimeHostContactPhone] = [],
        emails: [RuntimeHostContactEmail] = [],
        addresses: [RuntimeHostContactAddress] = [],
        organization: RuntimeHostContactOrganization = RuntimeHostContactOrganization(),
        note: String = ""
    ) {
        self.name = name
        self.phones = phones
        self.emails = emails
        self.addresses = addresses
        self.organization = organization
        self.note = note
    }
}

/// One Apple contact page returned by the host.
public struct RuntimeHostContactPage: Sendable, Hashable, Codable {
    /// The listed contacts for this page.
    public let contacts: [RuntimeHostContact]
    /// The opaque next-page cursor when available.
    public let nextCursor: String
    /// Whether more contacts are available.
    public let hasMore: Bool

    /// Create one contact page payload.
    public init(
        contacts: [RuntimeHostContact],
        nextCursor: String = "",
        hasMore: Bool = false
    ) {
        self.contacts = contacts
        self.nextCursor = nextCursor
        self.hasMore = hasMore
    }
}

/// One Apple contact-page response returned by the host.
public struct RuntimeHostContactPageResponse: Sendable, Hashable, Codable {
    /// The host status code.
    public let status: UInt32
    /// The returned contact page when available.
    public let page: RuntimeHostContactPage?

    /// Create one contact-page response.
    public init(
        status: UInt32,
        page: RuntimeHostContactPage? = nil
    ) {
        self.status = status
        self.page = page
    }
}

/// One Apple contact-read response returned by the host.
public struct RuntimeHostContactResponse: Sendable, Hashable, Codable {
    /// The host status code.
    public let status: UInt32
    /// The returned contact when available.
    public let contact: RuntimeHostContact?

    /// Create one contact-read response.
    public init(
        status: UInt32,
        contact: RuntimeHostContact? = nil
    ) {
        self.status = status
        self.contact = contact
    }
}

/// One Apple contact-create response returned by the host.
public struct RuntimeHostContactCreateResponse: Sendable, Hashable, Codable {
    /// The host status code.
    public let status: UInt32
    /// The created contact identifier when available.
    public let id: String?

    /// Create one contact-create response.
    public init(
        status: UInt32,
        id: String? = nil
    ) {
        self.status = status
        self.id = id
    }
}
