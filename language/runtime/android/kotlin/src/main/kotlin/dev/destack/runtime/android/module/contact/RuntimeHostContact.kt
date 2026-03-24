package dev.destack.runtime.android.module.contact

/**
 * The default contact page size when one query omits an explicit limit.
 */
internal const val runtimeHostContactDefaultPageLimit: Int = 50

/**
 * One Android contact query submitted by one runtime session.
 */
public data class RuntimeHostContactQuery(
    /**
     * The opaque cursor from one prior list or search call.
     */
    val cursor: String? = null,

    /**
     * The maximum returned contacts for this page when available.
     */
    val limit: Int? = null,

    /**
     * Whether phone values should be returned.
     */
    val includePhones: Boolean = false,

    /**
     * Whether email values should be returned.
     */
    val includeEmails: Boolean = false,

    /**
     * Whether postal-address values should be returned.
     */
    val includeAddresses: Boolean = false,

    /**
     * Whether organization metadata should be returned.
     */
    val includeOrganization: Boolean = false,

    /**
     * Whether note fields should be returned.
     */
    val includeNotes: Boolean = false,
)

/**
 * One structured Android contact-name payload.
 */
public data class RuntimeHostContactName(
    /**
     * The given or first name.
     */
    val givenName: String = "",

    /**
     * The middle name.
     */
    val middleName: String = "",

    /**
     * The family or last name.
     */
    val familyName: String = "",

    /**
     * The honorific prefix.
     */
    val prefix: String = "",

    /**
     * The honorific suffix.
     */
    val suffix: String = "",

    /**
     * The nickname.
     */
    val nickname: String = "",

    /**
     * The phonetic given name.
     */
    val phoneticGivenName: String = "",

    /**
     * The phonetic family name.
     */
    val phoneticFamilyName: String = "",
)

/**
 * One Android contact phone payload.
 */
public data class RuntimeHostContactPhone(
    /**
     * The user-visible label for this phone value.
     */
    val label: String = "",

    /**
     * The original phone number string.
     */
    val number: String = "",

    /**
     * The normalized phone number string when available.
     */
    val normalizedNumber: String = "",

    /**
     * Whether this phone value is marked as primary.
     */
    val primary: Boolean = false,
)

/**
 * One Android contact email payload.
 */
public data class RuntimeHostContactEmail(
    /**
     * The user-visible label for this email value.
     */
    val label: String = "",

    /**
     * The email address.
     */
    val address: String = "",

    /**
     * Whether this email value is marked as primary.
     */
    val primary: Boolean = false,
)

/**
 * One Android contact postal-address payload.
 */
public data class RuntimeHostContactAddress(
    /**
     * The user-visible label for this address value.
     */
    val label: String = "",

    /**
     * The street-line payload.
     */
    val street: String = "",

    /**
     * The city payload.
     */
    val city: String = "",

    /**
     * The region or state payload.
     */
    val region: String = "",

    /**
     * The postal-code payload.
     */
    val postalCode: String = "",

    /**
     * The country payload.
     */
    val country: String = "",

    /**
     * The country-code payload.
     */
    val countryCode: String = "",
)

/**
 * One Android contact organization payload.
 */
public data class RuntimeHostContactOrganization(
    /**
     * The company or organization name.
     */
    val company: String = "",

    /**
     * The department name.
     */
    val department: String = "",

    /**
     * The job title.
     */
    val title: String = "",
)

/**
 * One Android contact payload.
 */
public data class RuntimeHostContact(
    /**
     * The stable host contact identifier.
     */
    val id: String,

    /**
     * The structured name payload.
     */
    val name: RuntimeHostContactName = RuntimeHostContactName(),

    /**
     * The phone values.
     */
    val phones: List<RuntimeHostContactPhone> = emptyList(),

    /**
     * The email values.
     */
    val emails: List<RuntimeHostContactEmail> = emptyList(),

    /**
     * The postal-address values.
     */
    val addresses: List<RuntimeHostContactAddress> = emptyList(),

    /**
     * The organization metadata.
     */
    val organization: RuntimeHostContactOrganization = RuntimeHostContactOrganization(),

    /**
     * The contact note payload.
     */
    val note: String = "",
)

/**
 * One Android contact draft payload.
 */
public data class RuntimeHostContactDraft(
    /**
     * The structured name payload.
     */
    val name: RuntimeHostContactName = RuntimeHostContactName(),

    /**
     * The phone values.
     */
    val phones: List<RuntimeHostContactPhone> = emptyList(),

    /**
     * The email values.
     */
    val emails: List<RuntimeHostContactEmail> = emptyList(),

    /**
     * The postal-address values.
     */
    val addresses: List<RuntimeHostContactAddress> = emptyList(),

    /**
     * The organization metadata.
     */
    val organization: RuntimeHostContactOrganization = RuntimeHostContactOrganization(),

    /**
     * The contact note payload.
     */
    val note: String = "",
)

/**
 * One Android contact page returned by the host.
 */
public data class RuntimeHostContactPage(
    /**
     * The listed contacts for this page.
     */
    val contacts: List<RuntimeHostContact>,

    /**
     * The opaque next-page cursor when available.
     */
    val nextCursor: String = "",

    /**
     * Whether more contacts are available.
     */
    val hasMore: Boolean = false,
)

/**
 * One Android contact-page response returned by the host.
 */
public data class RuntimeHostContactPageResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned contact page when available.
     */
    val page: RuntimeHostContactPage? = null,
)

/**
 * One Android contact-read response returned by the host.
 */
public data class RuntimeHostContactResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned contact when available.
     */
    val contact: RuntimeHostContact? = null,
)

/**
 * One Android contact-create response returned by the host.
 */
public data class RuntimeHostContactCreateResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The created contact identifier when available.
     */
    val id: String? = null,
)
