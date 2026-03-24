package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.contact.RuntimeHostContact
import dev.destack.runtime.android.module.contact.RuntimeHostContactCreateResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactDraft
import dev.destack.runtime.android.module.contact.RuntimeHostContactOrganization
import dev.destack.runtime.android.module.contact.RuntimeHostContactName
import dev.destack.runtime.android.module.contact.RuntimeHostContactPage
import dev.destack.runtime.android.module.contact.RuntimeHostContactPageResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactQuery
import dev.destack.runtime.android.module.contact.RuntimeHostContactResponse

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise the contact bridge lane.
 */
class ContactBridgeTest {
    /**
     * Route contact requests through the attached runtime host.
     */
    @Test
    fun testContactRequestsRouteThroughRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val contactRequests = ContactRequestRecorder().also {
            it.listResponse = RuntimeHostContactPageResponse(
                status = 0,
                page = RuntimeHostContactPage(
                    contacts = listOf(sampleContact("contact-1")),
                    nextCursor = "cursor-2",
                    hasMore = true,
                ),
            )
            it.searchResponse = RuntimeHostContactPageResponse(
                status = 0,
                page = RuntimeHostContactPage(
                    contacts = listOf(sampleContact("contact-2")),
                ),
            )
            it.readResponse = RuntimeHostContactResponse(
                status = 0,
                contact = sampleContact("contact-3"),
            )
            it.createResponse = RuntimeHostContactCreateResponse(
                status = 0,
                id = "contact-4",
            )
            it.updateStatus = 0
            it.deleteStatus = 0
        }
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = contactRequests,
            intentRequests = IntentRequestRecorder(),
        )

        bridge.attach(runtimeHost)

        val listResponse = bridge.listContacts(
            cursor = "cursor-1",
            hasLimit = true,
            limit = 25,
            includePhones = true,
            includeEmails = false,
            includeAddresses = true,
            includeOrganization = false,
            includeNotes = true,
        )
        val searchResponse = bridge.searchContacts(
            queryText = "alice",
            cursor = null,
            hasLimit = false,
            limit = 0,
            includePhones = true,
            includeEmails = true,
            includeAddresses = false,
            includeOrganization = true,
            includeNotes = false,
        )
        val readResponse = bridge.readContact("contact-3")
        val createResponse = bridge.createContact(
            givenName = "Alice",
            middleName = "",
            familyName = "Example",
            prefix = "",
            suffix = "",
            nickname = "ally",
            phoneticGivenName = "",
            phoneticFamilyName = "",
            phones = emptyArray(),
            emails = emptyArray(),
            addresses = emptyArray(),
            company = "Destack",
            department = "Runtime",
            title = "Engineer",
            note = "note",
        )
        val updateStatus = bridge.updateContact(
            id = "contact-4",
            givenName = "Bob",
            middleName = "",
            familyName = "Example",
            prefix = "",
            suffix = "",
            nickname = "",
            phoneticGivenName = "",
            phoneticFamilyName = "",
            phones = emptyArray(),
            emails = emptyArray(),
            addresses = emptyArray(),
            company = "",
            department = "",
            title = "",
            note = "",
        )
        val deleteStatus = bridge.deleteContact("contact-4")

        assertEquals(
            listOf(
                RuntimeHostContactQuery(
                    cursor = "cursor-1",
                    limit = 25,
                    includePhones = true,
                    includeEmails = false,
                    includeAddresses = true,
                    includeOrganization = false,
                    includeNotes = true,
                ),
            ),
            contactRequests.listQueries,
        )
        assertEquals(
            listOf(
                "alice" to RuntimeHostContactQuery(
                    cursor = null,
                    limit = null,
                    includePhones = true,
                    includeEmails = true,
                    includeAddresses = false,
                    includeOrganization = true,
                    includeNotes = false,
                ),
            ),
            contactRequests.searchQueries,
        )
        assertEquals(listOf("contact-3"), contactRequests.readIdentifiers)
        assertEquals(listOf(sampleDraft("Alice")), contactRequests.createDrafts)
        assertEquals(listOf("contact-4" to sampleDraft("Bob")), contactRequests.updateCalls)
        assertEquals(listOf("contact-4"), contactRequests.deleteIdentifiers)
        assertEquals(0, listResponse.status)
        assertEquals(0, searchResponse.status)
        assertEquals(0, readResponse.status)
        assertEquals("contact-4", createResponse.id)
        assertEquals(0, updateStatus)
        assertEquals(0, deleteStatus)
    }
}

/**
 * Build one representative contact fixture.
 */
private fun sampleContact(
    id: String,
): RuntimeHostContact {
    return RuntimeHostContact(
        id = id,
        name = RuntimeHostContactName(
            givenName = "Alice",
            familyName = "Example",
            nickname = "ally",
        ),
        note = "note",
    )
}

/**
 * Build one representative contact draft fixture.
 */
private fun sampleDraft(
    givenName: String,
): RuntimeHostContactDraft {
    return RuntimeHostContactDraft(
        name = RuntimeHostContactName(
            givenName = givenName,
            familyName = "Example",
            nickname = if (givenName == "Alice") "ally" else "",
        ),
        organization = if (givenName == "Alice") {
            RuntimeHostContactOrganization(
                company = "Destack",
                department = "Runtime",
                title = "Engineer",
            )
        }
        else {
            RuntimeHostContactOrganization()
        },
        note = if (givenName == "Alice") "note" else "",
    )
}
