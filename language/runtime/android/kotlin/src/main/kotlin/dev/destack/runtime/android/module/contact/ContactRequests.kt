package dev.destack.runtime.android.module.contact

import dev.destack.runtime.android.core.hostStatusNotSupported

/**
 * The contact request surface for one Android runtime host.
 */
public interface ContactRequests {
    /**
     * List one page of contacts for one query.
     */
    public fun listContacts(
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse

    /**
     * Search contacts for one query string and one query shape.
     */
    public fun searchContacts(
        queryText: String,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse

    /**
     * Read one contact by stable identifier.
     */
    public fun readContact(
        id: String,
    ): RuntimeHostContactResponse

    /**
     * Create one contact and return its stable identifier.
     */
    public fun createContact(
        draft: RuntimeHostContactDraft,
    ): RuntimeHostContactCreateResponse

    /**
     * Update one contact by stable identifier.
     */
    public fun updateContact(
        id: String,
        draft: RuntimeHostContactDraft,
    ): Int

    /**
     * Delete one contact by stable identifier.
     */
    public fun deleteContact(
        id: String,
    ): Int
}

/**
 * The explicit unsupported contact request surface for one Android runtime host.
 */
public object UnsupportedContactRequests : ContactRequests {
    override fun listContacts(
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        return RuntimeHostContactPageResponse(status = hostStatusNotSupported)
    }

    override fun searchContacts(
        queryText: String,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        return RuntimeHostContactPageResponse(status = hostStatusNotSupported)
    }

    override fun readContact(
        id: String,
    ): RuntimeHostContactResponse {
        return RuntimeHostContactResponse(status = hostStatusNotSupported)
    }

    override fun createContact(
        draft: RuntimeHostContactDraft,
    ): RuntimeHostContactCreateResponse {
        return RuntimeHostContactCreateResponse(status = hostStatusNotSupported)
    }

    override fun updateContact(
        id: String,
        draft: RuntimeHostContactDraft,
    ): Int {
        return hostStatusNotSupported
    }

    override fun deleteContact(
        id: String,
    ): Int {
        return hostStatusNotSupported
    }
}
