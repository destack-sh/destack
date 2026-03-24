package dev.destack.runtime.android.bridge.contact

import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.contact.RuntimeHostContactCreateResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactDraft
import dev.destack.runtime.android.module.contact.RuntimeHostContactPageResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactQuery
import dev.destack.runtime.android.module.contact.RuntimeHostContactResponse

/**
 * One contact bridge lane for one attached Android runtime host.
 */
internal class ContactBridge {
    /**
     * List one page of contacts through the attached runtime host.
     */
    fun listContacts(
        runtimeHost: RuntimeHost,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        return runtimeHost.contactRequests.listContacts(query)
    }

    /**
     * Search contacts through the attached runtime host.
     */
    fun searchContacts(
        runtimeHost: RuntimeHost,
        queryText: String,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        return runtimeHost.contactRequests.searchContacts(queryText, query)
    }

    /**
     * Read one contact through the attached runtime host.
     */
    fun readContact(
        runtimeHost: RuntimeHost,
        id: String,
    ): RuntimeHostContactResponse {
        return runtimeHost.contactRequests.readContact(id)
    }

    /**
     * Create one contact through the attached runtime host.
     */
    fun createContact(
        runtimeHost: RuntimeHost,
        draft: RuntimeHostContactDraft,
    ): RuntimeHostContactCreateResponse {
        return runtimeHost.contactRequests.createContact(draft)
    }

    /**
     * Update one contact through the attached runtime host.
     */
    fun updateContact(
        runtimeHost: RuntimeHost,
        id: String,
        draft: RuntimeHostContactDraft,
    ): Int {
        return runtimeHost.contactRequests.updateContact(id, draft)
    }

    /**
     * Delete one contact through the attached runtime host.
     */
    fun deleteContact(
        runtimeHost: RuntimeHost,
        id: String,
    ): Int {
        return runtimeHost.contactRequests.deleteContact(id)
    }
}
