package dev.destack.runtime.android.module.contact

import android.Manifest
import android.content.ContentProviderOperation
import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.content.pm.PackageManager
import android.content.res.Resources
import android.provider.ContactsContract

import androidx.core.content.ContextCompat
import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.hostStatusPermissionDenied

private val supportedDataMimeTypes: List<String> = listOf(
    ContactsContract.CommonDataKinds.StructuredName.CONTENT_ITEM_TYPE,
    ContactsContract.CommonDataKinds.Phone.CONTENT_ITEM_TYPE,
    ContactsContract.CommonDataKinds.Email.CONTENT_ITEM_TYPE,
    ContactsContract.CommonDataKinds.StructuredPostal.CONTENT_ITEM_TYPE,
    ContactsContract.CommonDataKinds.Organization.CONTENT_ITEM_TYPE,
    ContactsContract.CommonDataKinds.Note.CONTENT_ITEM_TYPE,
)

/**
 * The Android contact request surface backed by `ContactsContract`.
 */
public class ContactsProviderRequests(
    context: Context,
) : ContactRequests {
    private val context: Context = context.applicationContext

    override fun list(
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        return list(
            context = context,
            query = query,
        )
    }

    override fun search(
        queryText: String,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        return search(
            context = context,
            queryText = queryText,
            query = query,
        )
    }

    override fun read(
        id: String,
    ): RuntimeHostContactResponse {
        return read(
            context = context,
            id = id,
        )
    }

    override fun create(
        draft: RuntimeHostContactDraft,
    ): RuntimeHostContactCreateResponse {
        return create(
            context = context,
            draft = draft,
        )
    }

    override fun update(
        id: String,
        draft: RuntimeHostContactDraft,
    ): Int {
        return update(
            context = context,
            id = id,
            draft = draft,
        )
    }

    override fun deleteContact(
        id: String,
    ): Int {
        return deleteContact(
            context = context,
            id = id,
        )
    }
}

/**
 * List one page of contacts through the Android contacts provider.
 */
private fun list(
    context: Context,
    query: RuntimeHostContactQuery,
): RuntimeHostContactPageResponse {
    // require read access before touching the provider
    if (!hasReadContactsPermission(context)) {
        return RuntimeHostContactPageResponse(status = hostStatusPermissionDenied)
    }

    // decode one stable page window
    val pageWindow = decodePageWindow(query)
        ?: return RuntimeHostContactPageResponse(status = hostStatusInvalidArgument)
    val contentResolver = context.contentResolver
    val contactIds = loadSortedContactIds(contentResolver)
        ?: return RuntimeHostContactPageResponse(status = hostStatusFailed)

    // page the stable identifier list before materializing the payloads
    val pageIds = contactIds
        .drop(pageWindow.offset)
        .take(pageWindow.limit + 1)
    val hasMore = pageIds.size > pageWindow.limit
    val responseIds = if (hasMore) {
        pageIds.dropLast(1)
    } else {
        pageIds
    }
    val contacts = responseIds.mapNotNull { contactId ->
        readRuntimeContact(
            contentResolver = contentResolver,
            resources = context.resources,
            contactId = contactId,
            query = query,
        )
    }
    val nextCursor = if (hasMore) {
        (pageWindow.offset + contacts.size).toString()
    } else {
        ""
    }

    return RuntimeHostContactPageResponse(
        status = hostStatusOk,
        page = RuntimeHostContactPage(
            contacts = contacts,
            nextCursor = nextCursor,
            hasMore = hasMore,
        ),
    )
}

/**
 * Search contacts through the Android contacts provider.
 */
private fun search(
    context: Context,
    queryText: String,
    query: RuntimeHostContactQuery,
): RuntimeHostContactPageResponse {
    // require read access before touching the provider
    if (!hasReadContactsPermission(context)) {
        return RuntimeHostContactPageResponse(status = hostStatusPermissionDenied)
    }

    // decode one stable page window
    val pageWindow = decodePageWindow(query)
        ?: return RuntimeHostContactPageResponse(status = hostStatusInvalidArgument)
    val contentResolver = context.contentResolver
    val contactIds = searchContactIds(
        contentResolver = contentResolver,
        queryText = queryText,
    )

    // materialize and sort the deduplicated search results before paging
    val contacts = contactIds.mapNotNull { contactId ->
        readRuntimeContact(
            contentResolver = contentResolver,
            resources = context.resources,
            contactId = contactId,
            query = query,
        )
    }
        .sortedBy { contact ->
            contactSortKey(contact)
        }
    val pageContacts = contacts
        .drop(pageWindow.offset)
        .take(pageWindow.limit + 1)
    val hasMore = pageContacts.size > pageWindow.limit
    val contactsResponse = if (hasMore) {
        pageContacts.dropLast(1)
    } else {
        pageContacts
    }
    val nextCursor = if (hasMore) {
        (pageWindow.offset + contactsResponse.size).toString()
    } else {
        ""
    }

    return RuntimeHostContactPageResponse(
        status = hostStatusOk,
        page = RuntimeHostContactPage(
            contacts = contactsResponse,
            nextCursor = nextCursor,
            hasMore = hasMore,
        ),
    )
}

/**
 * Read one contact by stable identifier through the Android contacts provider.
 */
private fun read(
    context: Context,
    id: String,
): RuntimeHostContactResponse {
    // require read access before touching the provider
    if (!hasReadContactsPermission(context)) {
        return RuntimeHostContactResponse(status = hostStatusPermissionDenied)
    }

    // decode one stable contact identifier
    val contactId = id.toLongOrNull()
        ?: return RuntimeHostContactResponse(status = hostStatusInvalidArgument)
    val contact = readRuntimeContact(
        contentResolver = context.contentResolver,
        resources = context.resources,
        contactId = contactId,
        query = fullContactQuery(),
    ) ?: return RuntimeHostContactResponse(status = hostStatusNotFound)

    return RuntimeHostContactResponse(
        status = hostStatusOk,
        contact = contact,
    )
}

/**
 * Create one contact through the Android contacts provider.
 */
private fun create(
    context: Context,
    draft: RuntimeHostContactDraft,
): RuntimeHostContactCreateResponse {
    // require write access before mutating the provider
    if (!hasWriteContactsPermission(context)) {
        return RuntimeHostContactCreateResponse(status = hostStatusPermissionDenied)
    }

    // build one insert batch around one new raw contact row
    val operations = arrayListOf<ContentProviderOperation>()
    operations += ContentProviderOperation.newInsert(ContactsContract.RawContacts.CONTENT_URI)
        .withValue(ContactsContract.RawContacts.ACCOUNT_TYPE, null)
        .withValue(ContactsContract.RawContacts.ACCOUNT_NAME, null)
        .build()
    appendDraftInsertOperations(
        operations = operations,
        rawContactBackReference = 0,
        draft = draft,
    )

    // apply the batch and resolve the created aggregate contact identifier
    return try {
        val results = context.contentResolver.applyBatch(
            ContactsContract.AUTHORITY,
            operations,
        )
        val rawContactUri = results.firstOrNull()?.uri
            ?: return RuntimeHostContactCreateResponse(status = hostStatusFailed)
        val rawContactId = ContentUris.parseId(rawContactUri)
        val contactId = resolveContactIdForRawContact(
            contentResolver = context.contentResolver,
            rawContactId = rawContactId,
        ) ?: return RuntimeHostContactCreateResponse(status = hostStatusFailed)

        RuntimeHostContactCreateResponse(
            status = hostStatusOk,
            id = contactId.toString(),
        )
    }
    catch (_: Exception) {
        RuntimeHostContactCreateResponse(status = hostStatusFailed)
    }
}

/**
 * Update one contact through the Android contacts provider.
 */
private fun update(
    context: Context,
    id: String,
    draft: RuntimeHostContactDraft,
): Int {
    // require write access before mutating the provider
    if (!hasWriteContactsPermission(context)) {
        return hostStatusPermissionDenied
    }

    // resolve one mutable raw-contact row for the aggregate contact
    val contactId = id.toLongOrNull() ?: return hostStatusInvalidArgument
    val rawContactId = resolveFirstRawContactId(
        contentResolver = context.contentResolver,
        contactId = contactId,
    ) ?: return hostStatusNotFound
    val operations = arrayListOf<ContentProviderOperation>()
    operations += ContentProviderOperation.newDelete(ContactsContract.Data.CONTENT_URI)
        .withSelection(
            "${ContactsContract.Data.RAW_CONTACT_ID} = ? AND ${ContactsContract.Data.MIMETYPE} IN (${supportedDataMimeTypes.indices.joinToString(",") { "?" }})",
            arrayOf(rawContactId.toString()) + supportedDataMimeTypes.toTypedArray(),
        )
        .build()
    appendDraftValueOperations(
        operations = operations,
        rawContactId = rawContactId,
        draft = draft,
    )

    // apply the destructive rewrite in one provider batch
    return try {
        context.contentResolver.applyBatch(
            ContactsContract.AUTHORITY,
            operations,
        )

        hostStatusOk
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

/**
 * Delete one contact through the Android contacts provider.
 */
private fun deleteContact(
    context: Context,
    id: String,
): Int {
    // require write access before mutating the provider
    if (!hasWriteContactsPermission(context)) {
        return hostStatusPermissionDenied
    }

    // delete the aggregate contact through the raw-contact table
    val contactId = id.toLongOrNull() ?: return hostStatusInvalidArgument

    return try {
        val deletedCount = context.contentResolver.delete(
            ContactsContract.RawContacts.CONTENT_URI,
            "${ContactsContract.RawContacts.CONTACT_ID} = ?",
            arrayOf(contactId.toString()),
        )

        if (deletedCount > 0) {
            hostStatusOk
        } else {
            hostStatusNotFound
        }
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

/**
 * Return whether read-contact permission is granted.
 */
private fun hasReadContactsPermission(
    context: Context,
): Boolean {
    return ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.READ_CONTACTS,
    ) == PackageManager.PERMISSION_GRANTED
}

/**
 * Return whether write-contact permission is granted.
 */
private fun hasWriteContactsPermission(
    context: Context,
): Boolean {
    return ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.WRITE_CONTACTS,
    ) == PackageManager.PERMISSION_GRANTED
}

/**
 * One decoded contact page window.
 */
private data class PageWindow(
    /**
     * The stable offset into the contact result list.
     */
    val offset: Int,

    /**
     * The decoded page limit.
     */
    val limit: Int,
)

/**
 * Decode one contact page window from one runtime query.
 */
private fun decodePageWindow(
    query: RuntimeHostContactQuery,
): PageWindow? {
    // decode one stable integer cursor when present
    val offset = query.cursor?.toIntOrNull() ?: 0
    if (query.cursor != null && offset < 0) {
        return null
    }

    // reject non-positive page limits loudly
    val limit = query.limit ?: runtimeHostContactDefaultPageLimit
    if (limit <= 0) {
        return null
    }

    return PageWindow(
        offset = offset,
        limit = limit,
    )
}

/**
 * Build one full contact query payload.
 */
private fun fullContactQuery(): RuntimeHostContactQuery {
    return RuntimeHostContactQuery(
        includePhones = true,
        includeEmails = true,
        includeAddresses = true,
        includeOrganization = true,
        includeNotes = true,
    )
}

/**
 * Load all stable contact identifiers in one user-visible order.
 */
private fun loadSortedContactIds(
    contentResolver: ContentResolver,
): List<Long>? {
    return try {
        contentResolver.query(
            ContactsContract.Contacts.CONTENT_URI,
            arrayOf(ContactsContract.Contacts._ID),
            null,
            null,
            "${ContactsContract.Contacts.DISPLAY_NAME_PRIMARY} COLLATE LOCALIZED ASC, ${ContactsContract.Contacts._ID} ASC",
        )?.use { cursor ->
            val contacts = mutableListOf<Long>()
            val idIndex = cursor.getColumnIndexOrThrow(ContactsContract.Contacts._ID)

            while (cursor.moveToNext()) {
                contacts += cursor.getLong(idIndex)
            }

            contacts
        }
    }
    catch (_: Exception) {
        null
    }
}

/**
 * Search stable contact identifiers across the primary Android contact indexes.
 */
private fun searchContactIds(
    contentResolver: ContentResolver,
    queryText: String,
): List<Long> {
    // return all contacts for one empty search input
    if (queryText.isBlank()) {
        return loadSortedContactIds(contentResolver) ?: emptyList()
    }

    val results = linkedSetOf<Long>()

    // search by display name
    appendContactIds(
        results = results,
        ids = searchNameContactIds(
            contentResolver = contentResolver,
            queryText = queryText,
        ),
    )

    // search by email address when the query looks like one
    if (queryText.contains('@')) {
        appendContactIds(
            results = results,
            ids = searchEmailContactIds(
                contentResolver = contentResolver,
                queryText = queryText,
            ),
        )
    }

    // search by phone number when the query contains digits
    if (queryText.any { character -> character.isDigit() }) {
        appendContactIds(
            results = results,
            ids = searchPhoneContactIds(
                contentResolver = contentResolver,
                queryText = queryText,
            ),
        )
    }

    // search by organization labels too
    appendContactIds(
        results = results,
        ids = searchOrganizationContactIds(
            contentResolver = contentResolver,
            queryText = queryText,
        ),
    )

    return results.toList()
}

/**
 * Append one contact-identifier sequence into one stable deduplicated set.
 */
private fun appendContactIds(
    results: MutableSet<Long>,
    ids: List<Long>,
) {
    for (id in ids) {
        results += id
    }
}

/**
 * Search contact identifiers by displayable name.
 */
private fun searchNameContactIds(
    contentResolver: ContentResolver,
    queryText: String,
): List<Long> {
    return try {
        val queryUri = ContactsContract.Contacts.CONTENT_FILTER_URI.buildUpon()
            .appendPath(queryText)
            .build()

        contentResolver.query(
            queryUri,
            arrayOf(ContactsContract.Contacts._ID),
            null,
            null,
            "${ContactsContract.Contacts.DISPLAY_NAME_PRIMARY} COLLATE LOCALIZED ASC, ${ContactsContract.Contacts._ID} ASC",
        )?.use { cursor ->
            readContactIds(
                cursor = cursor,
                column = ContactsContract.Contacts._ID,
            )
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Search contact identifiers by email address.
 */
private fun searchEmailContactIds(
    contentResolver: ContentResolver,
    queryText: String,
): List<Long> {
    return try {
        contentResolver.query(
            ContactsContract.CommonDataKinds.Email.CONTENT_URI,
            arrayOf(ContactsContract.CommonDataKinds.Email.CONTACT_ID),
            "${ContactsContract.CommonDataKinds.Email.ADDRESS} LIKE ?",
            arrayOf("%$queryText%"),
            "${ContactsContract.CommonDataKinds.Email.IS_PRIMARY} DESC, ${ContactsContract.CommonDataKinds.Email._ID} ASC",
        )?.use { cursor ->
            readContactIds(
                cursor = cursor,
                column = ContactsContract.CommonDataKinds.Email.CONTACT_ID,
            )
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Search contact identifiers by phone number.
 */
private fun searchPhoneContactIds(
    contentResolver: ContentResolver,
    queryText: String,
): List<Long> {
    val normalizedQuery = buildPhoneSearchQuery(queryText)

    return try {
        contentResolver.query(
            ContactsContract.CommonDataKinds.Phone.CONTENT_URI,
            arrayOf(ContactsContract.CommonDataKinds.Phone.CONTACT_ID),
            "${ContactsContract.CommonDataKinds.Phone.NUMBER} LIKE ? OR ${ContactsContract.CommonDataKinds.Phone.NORMALIZED_NUMBER} LIKE ?",
            arrayOf("%$normalizedQuery%", "%$normalizedQuery%"),
            "${ContactsContract.CommonDataKinds.Phone.IS_PRIMARY} DESC, ${ContactsContract.CommonDataKinds.Phone._ID} ASC",
        )?.use { cursor ->
            readContactIds(
                cursor = cursor,
                column = ContactsContract.CommonDataKinds.Phone.CONTACT_ID,
            )
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Search contact identifiers by organization metadata.
 */
private fun searchOrganizationContactIds(
    contentResolver: ContentResolver,
    queryText: String,
): List<Long> {
    return try {
        contentResolver.query(
            ContactsContract.Data.CONTENT_URI,
            arrayOf(ContactsContract.Data.CONTACT_ID),
            "${ContactsContract.Data.MIMETYPE} = ? AND (" +
                "${ContactsContract.CommonDataKinds.Organization.COMPANY} LIKE ? OR " +
                "${ContactsContract.CommonDataKinds.Organization.TITLE} LIKE ? OR " +
                "${ContactsContract.CommonDataKinds.Organization.DEPARTMENT} LIKE ?)",
            arrayOf(
                ContactsContract.CommonDataKinds.Organization.CONTENT_ITEM_TYPE,
                "%$queryText%",
                "%$queryText%",
                "%$queryText%",
            ),
            "${ContactsContract.Data.CONTACT_ID} ASC",
        )?.use { cursor ->
            readContactIds(
                cursor = cursor,
                column = ContactsContract.Data.CONTACT_ID,
            )
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Materialize one identifier list from one contact cursor.
 */
private fun readContactIds(
    cursor: android.database.Cursor,
    column: String,
): List<Long> {
    val contacts = mutableListOf<Long>()
    val idIndex = cursor.getColumnIndexOrThrow(column)

    while (cursor.moveToNext()) {
        contacts += cursor.getLong(idIndex)
    }

    return contacts
}

/**
 * Build one normalized phone search token.
 */
private fun buildPhoneSearchQuery(
    queryText: String,
): String {
    val normalized = buildString {
        for (character in queryText) {
            if (character.isDigit() || character == '+') {
                append(character)
            }
        }
    }

    return normalized.ifEmpty { queryText }
}

/**
 * Read one runtime contact payload through one aggregate contact identifier.
 */
private fun readRuntimeContact(
    contentResolver: ContentResolver,
    resources: Resources,
    contactId: Long,
    query: RuntimeHostContactQuery,
): RuntimeHostContact? {
    // require one live aggregate contact row before reading the payloads
    if (!contactExists(contentResolver, contactId)) {
        return null
    }

    // read the requested payload families through the data table
    val name = readContactName(
        contentResolver = contentResolver,
        contactId = contactId,
    )
    val phones = if (query.includePhones) {
        readContactPhones(
            contentResolver = contentResolver,
            resources = resources,
            contactId = contactId,
        )
    } else {
        emptyList()
    }
    val emails = if (query.includeEmails) {
        readContactEmails(
            contentResolver = contentResolver,
            resources = resources,
            contactId = contactId,
        )
    } else {
        emptyList()
    }
    val addresses = if (query.includeAddresses) {
        readContactAddresses(
            contentResolver = contentResolver,
            resources = resources,
            contactId = contactId,
        )
    } else {
        emptyList()
    }
    val organization = if (query.includeOrganization) {
        readContactOrganization(
            contentResolver = contentResolver,
            contactId = contactId,
        )
    } else {
        RuntimeHostContactOrganization()
    }
    val note = if (query.includeNotes) {
        readContactNote(
            contentResolver = contentResolver,
            contactId = contactId,
        )
    } else {
        ""
    }

    return RuntimeHostContact(
        id = contactId.toString(),
        name = name,
        phones = phones,
        emails = emails,
        addresses = addresses,
        organization = organization,
        note = note,
    )
}

/**
 * Return whether one aggregate contact exists.
 */
private fun contactExists(
    contentResolver: ContentResolver,
    contactId: Long,
): Boolean {
    return try {
        contentResolver.query(
            ContactsContract.Contacts.CONTENT_URI,
            arrayOf(ContactsContract.Contacts._ID),
            "${ContactsContract.Contacts._ID} = ?",
            arrayOf(contactId.toString()),
            null,
        )?.use { cursor ->
            cursor.moveToFirst()
        } == true
    }
    catch (_: Exception) {
        false
    }
}

/**
 * Read one structured name payload.
 */
private fun readContactName(
    contentResolver: ContentResolver,
    contactId: Long,
): RuntimeHostContactName {
    val nickname = readContactNickname(
        contentResolver = contentResolver,
        contactId = contactId,
    )

    return try {
        contentResolver.query(
            ContactsContract.Data.CONTENT_URI,
            arrayOf(
                ContactsContract.CommonDataKinds.StructuredName.GIVEN_NAME,
                ContactsContract.CommonDataKinds.StructuredName.MIDDLE_NAME,
                ContactsContract.CommonDataKinds.StructuredName.FAMILY_NAME,
                ContactsContract.CommonDataKinds.StructuredName.PREFIX,
                ContactsContract.CommonDataKinds.StructuredName.SUFFIX,
                ContactsContract.CommonDataKinds.StructuredName.PHONETIC_GIVEN_NAME,
                ContactsContract.CommonDataKinds.StructuredName.PHONETIC_FAMILY_NAME,
            ),
            "${ContactsContract.Data.CONTACT_ID} = ? AND ${ContactsContract.Data.MIMETYPE} = ?",
            arrayOf(
                contactId.toString(),
                ContactsContract.CommonDataKinds.StructuredName.CONTENT_ITEM_TYPE,
            ),
            "${ContactsContract.Data._ID} ASC",
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use RuntimeHostContactName()
            }

            RuntimeHostContactName(
                givenName = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.GIVEN_NAME),
                middleName = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.MIDDLE_NAME),
                familyName = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.FAMILY_NAME),
                prefix = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.PREFIX),
                suffix = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.SUFFIX),
                nickname = nickname,
                phoneticGivenName = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.PHONETIC_GIVEN_NAME),
                phoneticFamilyName = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredName.PHONETIC_FAMILY_NAME),
            )
        } ?: RuntimeHostContactName()
    }
    catch (_: Exception) {
        RuntimeHostContactName()
    }
}

/**
 * Read one nickname payload.
 */
private fun readContactNickname(
    contentResolver: ContentResolver,
    contactId: Long,
): String {
    return try {
        contentResolver.query(
            ContactsContract.Data.CONTENT_URI,
            arrayOf(ContactsContract.CommonDataKinds.Nickname.NAME),
            "${ContactsContract.Data.CONTACT_ID} = ? AND ${ContactsContract.Data.MIMETYPE} = ?",
            arrayOf(
                contactId.toString(),
                ContactsContract.CommonDataKinds.Nickname.CONTENT_ITEM_TYPE,
            ),
            "${ContactsContract.Data._ID} ASC",
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use ""
            }

            cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Nickname.NAME)
        } ?: ""
    }
    catch (_: Exception) {
        ""
    }
}

/**
 * Read one phone payload list.
 */
private fun readContactPhones(
    contentResolver: ContentResolver,
    resources: Resources,
    contactId: Long,
): List<RuntimeHostContactPhone> {
    return try {
        contentResolver.query(
            ContactsContract.CommonDataKinds.Phone.CONTENT_URI,
            arrayOf(
                ContactsContract.CommonDataKinds.Phone.TYPE,
                ContactsContract.CommonDataKinds.Phone.LABEL,
                ContactsContract.CommonDataKinds.Phone.NUMBER,
                ContactsContract.CommonDataKinds.Phone.NORMALIZED_NUMBER,
                ContactsContract.CommonDataKinds.Phone.IS_PRIMARY,
                ContactsContract.CommonDataKinds.Phone.IS_SUPER_PRIMARY,
            ),
            "${ContactsContract.CommonDataKinds.Phone.CONTACT_ID} = ?",
            arrayOf(contactId.toString()),
            "${ContactsContract.CommonDataKinds.Phone.IS_SUPER_PRIMARY} DESC, ${ContactsContract.CommonDataKinds.Phone.IS_PRIMARY} DESC, ${ContactsContract.CommonDataKinds.Phone._ID} ASC",
        )?.use { cursor ->
            val phones = mutableListOf<RuntimeHostContactPhone>()
            var index = 0

            while (cursor.moveToNext()) {
                val label = ContactsContract.CommonDataKinds.Phone.getTypeLabel(
                    resources,
                    cursor.getInt(cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.TYPE)),
                    cursor.getString(cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.LABEL)),
                ).toString()
                val number = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Phone.NUMBER)
                val normalizedNumber = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Phone.NORMALIZED_NUMBER)
                val isPrimary = cursor.intOrZero(ContactsContract.CommonDataKinds.Phone.IS_SUPER_PRIMARY) != 0 ||
                    cursor.intOrZero(ContactsContract.CommonDataKinds.Phone.IS_PRIMARY) != 0 ||
                    index == 0

                phones += RuntimeHostContactPhone(
                    label = label,
                    number = number,
                    normalizedNumber = if (normalizedNumber.isNotEmpty()) normalizedNumber else number,
                    primary = isPrimary,
                )
                index += 1
            }

            phones
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Read one email payload list.
 */
private fun readContactEmails(
    contentResolver: ContentResolver,
    resources: Resources,
    contactId: Long,
): List<RuntimeHostContactEmail> {
    return try {
        contentResolver.query(
            ContactsContract.CommonDataKinds.Email.CONTENT_URI,
            arrayOf(
                ContactsContract.CommonDataKinds.Email.TYPE,
                ContactsContract.CommonDataKinds.Email.LABEL,
                ContactsContract.CommonDataKinds.Email.ADDRESS,
                ContactsContract.CommonDataKinds.Email.IS_PRIMARY,
                ContactsContract.CommonDataKinds.Email.IS_SUPER_PRIMARY,
            ),
            "${ContactsContract.CommonDataKinds.Email.CONTACT_ID} = ?",
            arrayOf(contactId.toString()),
            "${ContactsContract.CommonDataKinds.Email.IS_SUPER_PRIMARY} DESC, ${ContactsContract.CommonDataKinds.Email.IS_PRIMARY} DESC, ${ContactsContract.CommonDataKinds.Email._ID} ASC",
        )?.use { cursor ->
            val emails = mutableListOf<RuntimeHostContactEmail>()
            var index = 0

            while (cursor.moveToNext()) {
                val label = ContactsContract.CommonDataKinds.Email.getTypeLabel(
                    resources,
                    cursor.getInt(cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Email.TYPE)),
                    cursor.getString(cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Email.LABEL)),
                ).toString()
                val address = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Email.ADDRESS)
                val isPrimary = cursor.intOrZero(ContactsContract.CommonDataKinds.Email.IS_SUPER_PRIMARY) != 0 ||
                    cursor.intOrZero(ContactsContract.CommonDataKinds.Email.IS_PRIMARY) != 0 ||
                    index == 0

                emails += RuntimeHostContactEmail(
                    label = label,
                    address = address,
                    primary = isPrimary,
                )
                index += 1
            }

            emails
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Read one address payload list.
 */
private fun readContactAddresses(
    contentResolver: ContentResolver,
    resources: Resources,
    contactId: Long,
): List<RuntimeHostContactAddress> {
    return try {
        contentResolver.query(
            ContactsContract.CommonDataKinds.StructuredPostal.CONTENT_URI,
            arrayOf(
                ContactsContract.CommonDataKinds.StructuredPostal.TYPE,
                ContactsContract.CommonDataKinds.StructuredPostal.LABEL,
                ContactsContract.CommonDataKinds.StructuredPostal.STREET,
                ContactsContract.CommonDataKinds.StructuredPostal.CITY,
                ContactsContract.CommonDataKinds.StructuredPostal.REGION,
                ContactsContract.CommonDataKinds.StructuredPostal.POSTCODE,
                ContactsContract.CommonDataKinds.StructuredPostal.COUNTRY,
            ),
            "${ContactsContract.CommonDataKinds.StructuredPostal.CONTACT_ID} = ?",
            arrayOf(contactId.toString()),
            "${ContactsContract.CommonDataKinds.StructuredPostal._ID} ASC",
        )?.use { cursor ->
            val addresses = mutableListOf<RuntimeHostContactAddress>()

            while (cursor.moveToNext()) {
                val label = ContactsContract.CommonDataKinds.StructuredPostal.getTypeLabel(
                    resources,
                    cursor.getInt(cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.StructuredPostal.TYPE)),
                    cursor.getString(cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.StructuredPostal.LABEL)),
                ).toString()

                addresses += RuntimeHostContactAddress(
                    label = label,
                    street = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredPostal.STREET),
                    city = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredPostal.CITY),
                    region = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredPostal.REGION),
                    postalCode = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredPostal.POSTCODE),
                    country = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.StructuredPostal.COUNTRY),
                    countryCode = "",
                )
            }

            addresses
        } ?: emptyList()
    }
    catch (_: Exception) {
        emptyList()
    }
}

/**
 * Read one organization payload.
 */
private fun readContactOrganization(
    contentResolver: ContentResolver,
    contactId: Long,
): RuntimeHostContactOrganization {
    return try {
        contentResolver.query(
            ContactsContract.Data.CONTENT_URI,
            arrayOf(
                ContactsContract.CommonDataKinds.Organization.COMPANY,
                ContactsContract.CommonDataKinds.Organization.DEPARTMENT,
                ContactsContract.CommonDataKinds.Organization.TITLE,
            ),
            "${ContactsContract.Data.CONTACT_ID} = ? AND ${ContactsContract.Data.MIMETYPE} = ?",
            arrayOf(
                contactId.toString(),
                ContactsContract.CommonDataKinds.Organization.CONTENT_ITEM_TYPE,
            ),
            "${ContactsContract.Data._ID} ASC",
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use RuntimeHostContactOrganization()
            }

            RuntimeHostContactOrganization(
                company = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Organization.COMPANY),
                department = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Organization.DEPARTMENT),
                title = cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Organization.TITLE),
            )
        } ?: RuntimeHostContactOrganization()
    }
    catch (_: Exception) {
        RuntimeHostContactOrganization()
    }
}

/**
 * Read one note payload.
 */
private fun readContactNote(
    contentResolver: ContentResolver,
    contactId: Long,
): String {
    return try {
        contentResolver.query(
            ContactsContract.Data.CONTENT_URI,
            arrayOf(ContactsContract.CommonDataKinds.Note.NOTE),
            "${ContactsContract.Data.CONTACT_ID} = ? AND ${ContactsContract.Data.MIMETYPE} = ?",
            arrayOf(
                contactId.toString(),
                ContactsContract.CommonDataKinds.Note.CONTENT_ITEM_TYPE,
            ),
            "${ContactsContract.Data._ID} ASC",
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use ""
            }

            cursor.stringOrEmpty(ContactsContract.CommonDataKinds.Note.NOTE)
        } ?: ""
    }
    catch (_: Exception) {
        ""
    }
}

/**
 * Append one draft insert batch through one raw-contact back reference.
 */
private fun appendDraftInsertOperations(
    operations: MutableList<ContentProviderOperation>,
    rawContactBackReference: Int,
    draft: RuntimeHostContactDraft,
) {
    // name row
    appendStructuredNameInsert(
        operations = operations,
        rawContactBackReference = rawContactBackReference,
        draft = draft,
    )
    appendNicknameInsert(
        operations = operations,
        rawContactBackReference = rawContactBackReference,
        draft = draft,
    )

    // phone rows
    for (phone in draft.phones) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Phone.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Phone.TYPE, phoneType(phone.label))
            .withValue(ContactsContract.CommonDataKinds.Phone.LABEL, customLabel(phone.label, phoneType(phone.label)))
            .withValue(ContactsContract.CommonDataKinds.Phone.NUMBER, phone.number)
            .withValue(ContactsContract.CommonDataKinds.Phone.NORMALIZED_NUMBER, phone.normalizedNumber.ifEmpty { null })
            .withValue(ContactsContract.CommonDataKinds.Phone.IS_PRIMARY, if (phone.primary) 1 else 0)
            .withValue(ContactsContract.CommonDataKinds.Phone.IS_SUPER_PRIMARY, if (phone.primary) 1 else 0)
            .build()
    }

    // email rows
    for (email in draft.emails) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Email.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Email.TYPE, emailType(email.label))
            .withValue(ContactsContract.CommonDataKinds.Email.LABEL, customLabel(email.label, emailType(email.label)))
            .withValue(ContactsContract.CommonDataKinds.Email.ADDRESS, email.address)
            .withValue(ContactsContract.CommonDataKinds.Email.IS_PRIMARY, if (email.primary) 1 else 0)
            .withValue(ContactsContract.CommonDataKinds.Email.IS_SUPER_PRIMARY, if (email.primary) 1 else 0)
            .build()
    }

    // address rows
    for (address in draft.addresses) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.StructuredPostal.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.TYPE, addressType(address.label))
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.LABEL, customLabel(address.label, addressType(address.label)))
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.STREET, address.street)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.CITY, address.city)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.REGION, address.region)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.POSTCODE, address.postalCode)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.COUNTRY, address.country)
            .build()
    }

    // organization row
    if (draft.organization != RuntimeHostContactOrganization()) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Organization.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Organization.COMPANY, draft.organization.company)
            .withValue(ContactsContract.CommonDataKinds.Organization.DEPARTMENT, draft.organization.department)
            .withValue(ContactsContract.CommonDataKinds.Organization.TITLE, draft.organization.title)
            .build()
    }

    // note row
    if (draft.note.isNotEmpty()) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Note.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Note.NOTE, draft.note)
            .build()
    }

}

/**
 * Append one draft rewrite batch through one concrete raw-contact identifier.
 */
private fun appendDraftValueOperations(
    operations: MutableList<ContentProviderOperation>,
    rawContactId: Long,
    draft: RuntimeHostContactDraft,
) {
    // rewrite the structured payload rows directly against the resolved raw contact
    appendStructuredNameInsert(
        operations = operations,
        rawContactId = rawContactId,
        draft = draft,
    )
    appendNicknameInsert(
        operations = operations,
        rawContactId = rawContactId,
        draft = draft,
    )

    for (phone in draft.phones) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Phone.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Phone.TYPE, phoneType(phone.label))
            .withValue(ContactsContract.CommonDataKinds.Phone.LABEL, customLabel(phone.label, phoneType(phone.label)))
            .withValue(ContactsContract.CommonDataKinds.Phone.NUMBER, phone.number)
            .withValue(ContactsContract.CommonDataKinds.Phone.NORMALIZED_NUMBER, phone.normalizedNumber.ifEmpty { null })
            .withValue(ContactsContract.CommonDataKinds.Phone.IS_PRIMARY, if (phone.primary) 1 else 0)
            .withValue(ContactsContract.CommonDataKinds.Phone.IS_SUPER_PRIMARY, if (phone.primary) 1 else 0)
            .build()
    }

    for (email in draft.emails) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Email.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Email.TYPE, emailType(email.label))
            .withValue(ContactsContract.CommonDataKinds.Email.LABEL, customLabel(email.label, emailType(email.label)))
            .withValue(ContactsContract.CommonDataKinds.Email.ADDRESS, email.address)
            .withValue(ContactsContract.CommonDataKinds.Email.IS_PRIMARY, if (email.primary) 1 else 0)
            .withValue(ContactsContract.CommonDataKinds.Email.IS_SUPER_PRIMARY, if (email.primary) 1 else 0)
            .build()
    }

    for (address in draft.addresses) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.StructuredPostal.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.TYPE, addressType(address.label))
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.LABEL, customLabel(address.label, addressType(address.label)))
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.STREET, address.street)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.CITY, address.city)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.REGION, address.region)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.POSTCODE, address.postalCode)
            .withValue(ContactsContract.CommonDataKinds.StructuredPostal.COUNTRY, address.country)
            .build()
    }

    if (draft.organization != RuntimeHostContactOrganization()) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Organization.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Organization.COMPANY, draft.organization.company)
            .withValue(ContactsContract.CommonDataKinds.Organization.DEPARTMENT, draft.organization.department)
            .withValue(ContactsContract.CommonDataKinds.Organization.TITLE, draft.organization.title)
            .build()
    }

    if (draft.note.isNotEmpty()) {
        operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
            .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
            .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Note.CONTENT_ITEM_TYPE)
            .withValue(ContactsContract.CommonDataKinds.Note.NOTE, draft.note)
            .build()
    }
}

/**
 * Append one structured-name insert operation when the draft carries visible name fields.
 */
private fun appendStructuredNameInsert(
    operations: MutableList<ContentProviderOperation>,
    rawContactBackReference: Int,
    draft: RuntimeHostContactDraft,
) {
    val name = draft.name
    val hasNamePayload = name != RuntimeHostContactName()

    if (!hasNamePayload) {
        return
    }

    operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
        .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
        .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.StructuredName.CONTENT_ITEM_TYPE)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.GIVEN_NAME, name.givenName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.MIDDLE_NAME, name.middleName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.FAMILY_NAME, name.familyName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.PREFIX, name.prefix)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.SUFFIX, name.suffix)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.PHONETIC_GIVEN_NAME, name.phoneticGivenName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.PHONETIC_FAMILY_NAME, name.phoneticFamilyName)
        .build()
}

/**
 * Append one structured-name insert operation through one concrete raw-contact identifier.
 */
private fun appendStructuredNameInsert(
    operations: MutableList<ContentProviderOperation>,
    rawContactId: Long,
    draft: RuntimeHostContactDraft,
) {
    val name = draft.name
    val hasNamePayload = name != RuntimeHostContactName()

    if (!hasNamePayload) {
        return
    }

    operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
        .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
        .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.StructuredName.CONTENT_ITEM_TYPE)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.GIVEN_NAME, name.givenName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.MIDDLE_NAME, name.middleName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.FAMILY_NAME, name.familyName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.PREFIX, name.prefix)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.SUFFIX, name.suffix)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.PHONETIC_GIVEN_NAME, name.phoneticGivenName)
        .withValue(ContactsContract.CommonDataKinds.StructuredName.PHONETIC_FAMILY_NAME, name.phoneticFamilyName)
        .build()
}

/**
 * Append one nickname insert operation through one raw-contact back reference.
 */
private fun appendNicknameInsert(
    operations: MutableList<ContentProviderOperation>,
    rawContactBackReference: Int,
    draft: RuntimeHostContactDraft,
) {
    val nickname = draft.name.nickname

    if (nickname.isBlank()) {
        return
    }

    operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
        .withValueBackReference(ContactsContract.Data.RAW_CONTACT_ID, rawContactBackReference)
        .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Nickname.CONTENT_ITEM_TYPE)
        .withValue(ContactsContract.CommonDataKinds.Nickname.NAME, nickname)
        .build()
}

/**
 * Append one nickname insert operation through one concrete raw-contact identifier.
 */
private fun appendNicknameInsert(
    operations: MutableList<ContentProviderOperation>,
    rawContactId: Long,
    draft: RuntimeHostContactDraft,
) {
    val nickname = draft.name.nickname

    if (nickname.isBlank()) {
        return
    }

    operations += ContentProviderOperation.newInsert(ContactsContract.Data.CONTENT_URI)
        .withValue(ContactsContract.Data.RAW_CONTACT_ID, rawContactId)
        .withValue(ContactsContract.Data.MIMETYPE, ContactsContract.CommonDataKinds.Nickname.CONTENT_ITEM_TYPE)
        .withValue(ContactsContract.CommonDataKinds.Nickname.NAME, nickname)
        .build()
}

/**
 * Resolve one aggregate contact identifier from one raw-contact identifier.
 */
private fun resolveContactIdForRawContact(
    contentResolver: ContentResolver,
    rawContactId: Long,
): Long? {
    return try {
        contentResolver.query(
            ContactsContract.RawContacts.CONTENT_URI,
            arrayOf(ContactsContract.RawContacts.CONTACT_ID),
            "${ContactsContract.RawContacts._ID} = ?",
            arrayOf(rawContactId.toString()),
            null,
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use null
            }

            cursor.getLong(cursor.getColumnIndexOrThrow(ContactsContract.RawContacts.CONTACT_ID))
        }
    }
    catch (_: Exception) {
        null
    }
}

/**
 * Resolve one mutable raw-contact identifier for one aggregate contact.
 */
private fun resolveFirstRawContactId(
    contentResolver: ContentResolver,
    contactId: Long,
): Long? {
    return try {
        contentResolver.query(
            ContactsContract.RawContacts.CONTENT_URI,
            arrayOf(ContactsContract.RawContacts._ID),
            "${ContactsContract.RawContacts.CONTACT_ID} = ? AND ${ContactsContract.RawContacts.DELETED} = 0",
            arrayOf(contactId.toString()),
            "${ContactsContract.RawContacts._ID} ASC",
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use null
            }

            cursor.getLong(cursor.getColumnIndexOrThrow(ContactsContract.RawContacts._ID))
        }
    }
    catch (_: Exception) {
        null
    }
}

/**
 * Build one stable contact sort key.
 */
private fun contactSortKey(
    contact: RuntimeHostContact,
): String {
    val visibleName = listOf(
        contact.name.givenName,
        contact.name.middleName,
        contact.name.familyName,
        contact.organization.company,
    )
        .filter { value -> value.isNotBlank() }
        .joinToString(" ")

    return "${visibleName.lowercase()} ${contact.id}"
}

/**
 * Return one string column or the empty string when the provider value is null.
 */
private fun android.database.Cursor.stringOrEmpty(
    column: String,
): String {
    val index = getColumnIndexOrThrow(column)

    if (isNull(index)) {
        return ""
    }

    return getString(index) ?: ""
}

/**
 * Return one integer column or zero when the provider value is null.
 */
private fun android.database.Cursor.intOrZero(
    column: String,
): Int {
    val index = getColumnIndexOrThrow(column)

    if (isNull(index)) {
        return 0
    }

    return getInt(index)
}

/**
 * Map one runtime phone label onto one Android phone type.
 */
private fun phoneType(
    label: String,
): Int {
    return when (label.lowercase()) {
        "mobile" -> ContactsContract.CommonDataKinds.Phone.TYPE_MOBILE
        "home" -> ContactsContract.CommonDataKinds.Phone.TYPE_HOME
        "work" -> ContactsContract.CommonDataKinds.Phone.TYPE_WORK
        "main" -> ContactsContract.CommonDataKinds.Phone.TYPE_MAIN
        "fax work" -> ContactsContract.CommonDataKinds.Phone.TYPE_FAX_WORK
        "fax home" -> ContactsContract.CommonDataKinds.Phone.TYPE_FAX_HOME
        "pager" -> ContactsContract.CommonDataKinds.Phone.TYPE_PAGER
        else -> ContactsContract.CommonDataKinds.Phone.TYPE_CUSTOM
    }
}

/**
 * Map one runtime email label onto one Android email type.
 */
private fun emailType(
    label: String,
): Int {
    return when (label.lowercase()) {
        "home" -> ContactsContract.CommonDataKinds.Email.TYPE_HOME
        "work" -> ContactsContract.CommonDataKinds.Email.TYPE_WORK
        "mobile" -> ContactsContract.CommonDataKinds.Email.TYPE_MOBILE
        "other" -> ContactsContract.CommonDataKinds.Email.TYPE_OTHER
        else -> ContactsContract.CommonDataKinds.Email.TYPE_CUSTOM
    }
}

/**
 * Map one runtime address label onto one Android postal-address type.
 */
private fun addressType(
    label: String,
): Int {
    return when (label.lowercase()) {
        "home" -> ContactsContract.CommonDataKinds.StructuredPostal.TYPE_HOME
        "work" -> ContactsContract.CommonDataKinds.StructuredPostal.TYPE_WORK
        "other" -> ContactsContract.CommonDataKinds.StructuredPostal.TYPE_OTHER
        else -> ContactsContract.CommonDataKinds.StructuredPostal.TYPE_CUSTOM
    }
}

/**
 * Return one custom label only for one custom typed row.
 */
private fun customLabel(
    label: String,
    type: Int,
): String? {
    if (label.isBlank()) {
        return null
    }

    val customTypes = setOf(
        ContactsContract.CommonDataKinds.Phone.TYPE_CUSTOM,
        ContactsContract.CommonDataKinds.Email.TYPE_CUSTOM,
        ContactsContract.CommonDataKinds.StructuredPostal.TYPE_CUSTOM,
    )

    return if (type in customTypes) {
        label
    } else {
        null
    }
}
