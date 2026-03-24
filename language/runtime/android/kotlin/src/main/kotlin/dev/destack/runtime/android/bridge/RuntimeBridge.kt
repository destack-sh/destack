package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.bridge.calendar.CalendarBridge
import dev.destack.runtime.android.bridge.contact.ContactBridge
import dev.destack.runtime.android.bridge.core.MainThreadBridge
import dev.destack.runtime.android.bridge.document.DocumentBridge
import dev.destack.runtime.android.bridge.intent.IntentBridge
import dev.destack.runtime.android.bridge.location.LocationBridge
import dev.destack.runtime.android.bridge.media.MediaBridge
import dev.destack.runtime.android.bridge.notification.NotificationBridge
import dev.destack.runtime.android.bridge.permission.PermissionBridge
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarAttendee
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarAvailability
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventCreateResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventDraft
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventListResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventQuery
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarListResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarRecurrenceRule
import dev.destack.runtime.android.module.contact.RuntimeHostContactAddress
import dev.destack.runtime.android.module.contact.RuntimeHostContactCreateResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactDraft
import dev.destack.runtime.android.module.contact.RuntimeHostContactEmail
import dev.destack.runtime.android.module.contact.RuntimeHostContactName
import dev.destack.runtime.android.module.contact.RuntimeHostContactOrganization
import dev.destack.runtime.android.module.contact.RuntimeHostContactPage
import dev.destack.runtime.android.module.contact.RuntimeHostContactPageResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactPhone
import dev.destack.runtime.android.module.contact.RuntimeHostContactQuery
import dev.destack.runtime.android.module.contact.RuntimeHostContactResponse
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.location.RuntimeHostLocationAccuracy
import dev.destack.runtime.android.module.location.RuntimeHostLocationLastKnownResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.location.RuntimeHostLocationServicesResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationWatchOptions
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaReadResponse
import dev.destack.runtime.android.module.notification.NotificationEvents
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent

/**
 * One runtime-backed bridge for one attached Android runtime host.
 */
public class RuntimeBridge internal constructor(
    /**
     * The runtime session routed through this bridge.
     */
    public val sessionHandle: HostSessionHandle,
    private val bindings: RuntimeAbi = ProcessRuntimeAbi,
    private val notificationTimestampNs: () -> Long = System::nanoTime,
) : NotificationEvents {
    private val mainThreadBridge = MainThreadBridge()
    private val documentBridge = DocumentBridge(
        sessionHandle = sessionHandle,
        bindings = bindings,
        mainThreadBridge = mainThreadBridge,
    )
    private val permissionBridge = PermissionBridge(
        sessionHandle = sessionHandle,
        bindings = bindings,
        mainThreadBridge = mainThreadBridge,
    )
    private val calendarBridge = CalendarBridge()
    private val contactBridge = ContactBridge()
    private val intentBridge = IntentBridge(
        sessionHandle = sessionHandle,
        bindings = bindings,
    )
    private val locationBridge = LocationBridge(
        sessionHandle = sessionHandle,
        bindings = bindings,
    )
    private val mediaBridge = MediaBridge(
        sessionHandle = sessionHandle,
    )
    private val notificationBridge = NotificationBridge(
        sessionHandle = sessionHandle,
        bindings = bindings,
        timestampNs = notificationTimestampNs,
    )
    private var runtimeHost: RuntimeHost? = null

    /**
     * Create one runtime bridge for one runtime session using the process ABI.
     */
    public constructor(
        sessionHandle: HostSessionHandle,
    ) : this(sessionHandle = sessionHandle, bindings = ProcessRuntimeAbi)

    /**
     * Attach one runtime host and register the bridge callback lanes.
     */
    public fun attach(
        runtimeHost: RuntimeHost,
    ) {
        require(runtimeHost.sessionHandle == sessionHandle) {
            "runtime bridge session handle does not match the attached runtime host"
        }

        this.runtimeHost = runtimeHost

        // register one session callback table after every lane is attached
        val attachStatus = bindings.attachBridge(
            sessionHandle = sessionHandle,
            bridge = this,
        )
        if (attachStatus != hostStatusOk) {
            this.runtimeHost = null
            throw IllegalStateException(
                "runtime bridge could not attach runtime abi bindings: $attachStatus",
            )
        }
    }

    /**
     * Remove the bridge registration for this runtime session.
     */
    public fun detach() {
        bindings.detachBridge(sessionHandle)
        runtimeHost = null
    }

    /**
     * Resolve the live runtime host for one bridge callback.
     */
    private inline fun <T> withRuntimeHost(
        onMissing: () -> T,
        body: (RuntimeHost) -> T,
    ): T {
        val runtimeHost = runtimeHost
            ?: return onMissing()

        return body(runtimeHost)
    }

    /**
     * Send one permission result into the runtime ingress path.
     */
    public fun sendPermissionEvent(
        event: RuntimeHostPermissionEvent,
    ) {
        permissionBridge.sendPermissionEvent(event)
    }

    /**
     * Send one document result into the runtime ingress path.
     */
    public fun sendDocumentResult(
        result: RuntimeHostDocumentResult,
    ) {
        documentBridge.sendDocumentResult(result)
    }

    /**
     * Send one intent event into the runtime ingress path.
     */
    public fun sendIntentEvent(
        event: RuntimeHostIntentEvent,
    ) {
        intentBridge.sendIntentEvent(event)
    }

    /**
     * Send one location sample into the runtime ingress path.
     */
    public fun sendLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    ) {
        locationBridge.sendLocationSample(watchId, sample)
    }

    /**
     * Send one notification event into the runtime ingress path.
     */
    override fun sendNotificationEvent(
        event: RuntimeHostNotificationEvent,
    ) {
        notificationBridge.sendNotificationEvent(event)
    }

    /**
     * Submit one document request decoded from one runtime callback payload.
     */
    @JvmName("submitDocumentRequest")
    internal fun submitDocumentRequest(
        requestId: Long,
        mimeTypes: Array<String>,
        extensions: Array<String>,
        allowsMultipleSelection: Boolean,
        allowsDirectorySelection: Boolean,
        copiesToSandbox: Boolean,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            documentBridge.submitRequest(
                runtimeHost = runtimeHost,
                requestId = requestId,
                mimeTypes = mimeTypes,
                extensions = extensions,
                allowsMultipleSelection = allowsMultipleSelection,
                allowsDirectorySelection = allowsDirectorySelection,
                copiesToSandbox = copiesToSandbox,
            )
        }
    }

    /**
     * Submit one permission request decoded from one runtime callback payload.
     */
    @JvmName("submitPermissionRequest")
    internal fun submitPermissionRequest(
        requestId: Long,
        permissions: Array<String>,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            permissionBridge.submitRequest(
                runtimeHost = runtimeHost,
                requestId = requestId,
                permissions = permissions,
            )
        }
    }

    /**
     * Open the native permission settings surface for one runtime callback.
     */
    @JvmName("openPermissionSettings")
    internal fun openPermissionSettings(): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            permissionBridge.openSettings(runtimeHost)
        }
    }

    /**
     * List one page of contacts for one runtime callback.
     */
    @JvmName("listContacts")
    internal fun listContacts(
        cursor: String?,
        hasLimit: Boolean,
        limit: Int,
        includePhones: Boolean,
        includeEmails: Boolean,
        includeAddresses: Boolean,
        includeOrganization: Boolean,
        includeNotes: Boolean,
    ): RuntimeHostContactPageResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostContactPageResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            contactBridge.listContacts(
                runtimeHost = runtimeHost,
                query = RuntimeHostContactQuery(
                    cursor = cursor,
                    limit = if (hasLimit) limit else null,
                    includePhones = includePhones,
                    includeEmails = includeEmails,
                    includeAddresses = includeAddresses,
                    includeOrganization = includeOrganization,
                    includeNotes = includeNotes,
                ),
            )
        }
    }

    /**
     * Search contacts for one runtime callback.
     */
    @JvmName("searchContacts")
    internal fun searchContacts(
        queryText: String,
        cursor: String?,
        hasLimit: Boolean,
        limit: Int,
        includePhones: Boolean,
        includeEmails: Boolean,
        includeAddresses: Boolean,
        includeOrganization: Boolean,
        includeNotes: Boolean,
    ): RuntimeHostContactPageResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostContactPageResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            contactBridge.searchContacts(
                runtimeHost = runtimeHost,
                queryText = queryText,
                query = RuntimeHostContactQuery(
                    cursor = cursor,
                    limit = if (hasLimit) limit else null,
                    includePhones = includePhones,
                    includeEmails = includeEmails,
                    includeAddresses = includeAddresses,
                    includeOrganization = includeOrganization,
                    includeNotes = includeNotes,
                ),
            )
        }
    }

    /**
     * Read one contact for one runtime callback.
     */
    @JvmName("readContact")
    internal fun readContact(
        id: String,
    ): RuntimeHostContactResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostContactResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            contactBridge.readContact(runtimeHost, id)
        }
    }

    /**
     * Create one contact for one runtime callback.
     */
    @JvmName("createContact")
    internal fun createContact(
        givenName: String,
        middleName: String,
        familyName: String,
        prefix: String,
        suffix: String,
        nickname: String,
        phoneticGivenName: String,
        phoneticFamilyName: String,
        phones: Array<RuntimeHostContactPhone>,
        emails: Array<RuntimeHostContactEmail>,
        addresses: Array<RuntimeHostContactAddress>,
        company: String,
        department: String,
        title: String,
        note: String,
    ): RuntimeHostContactCreateResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostContactCreateResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            contactBridge.createContact(
                runtimeHost = runtimeHost,
                draft = RuntimeHostContactDraft(
                    name = RuntimeHostContactName(
                        givenName = givenName,
                        middleName = middleName,
                        familyName = familyName,
                        prefix = prefix,
                        suffix = suffix,
                        nickname = nickname,
                        phoneticGivenName = phoneticGivenName,
                        phoneticFamilyName = phoneticFamilyName,
                    ),
                    phones = phones.asList(),
                    emails = emails.asList(),
                    addresses = addresses.asList(),
                    organization = RuntimeHostContactOrganization(
                        company = company,
                        department = department,
                        title = title,
                    ),
                    note = note,
                ),
            )
        }
    }

    /**
     * Update one contact for one runtime callback.
     */
    @JvmName("updateContact")
    internal fun updateContact(
        id: String,
        givenName: String,
        middleName: String,
        familyName: String,
        prefix: String,
        suffix: String,
        nickname: String,
        phoneticGivenName: String,
        phoneticFamilyName: String,
        phones: Array<RuntimeHostContactPhone>,
        emails: Array<RuntimeHostContactEmail>,
        addresses: Array<RuntimeHostContactAddress>,
        company: String,
        department: String,
        title: String,
        note: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            contactBridge.updateContact(
                runtimeHost = runtimeHost,
                id = id,
                draft = RuntimeHostContactDraft(
                    name = RuntimeHostContactName(
                        givenName = givenName,
                        middleName = middleName,
                        familyName = familyName,
                        prefix = prefix,
                        suffix = suffix,
                        nickname = nickname,
                        phoneticGivenName = phoneticGivenName,
                        phoneticFamilyName = phoneticFamilyName,
                    ),
                    phones = phones.asList(),
                    emails = emails.asList(),
                    addresses = addresses.asList(),
                    organization = RuntimeHostContactOrganization(
                        company = company,
                        department = department,
                        title = title,
                    ),
                    note = note,
                ),
            )
        }
    }

    /**
     * Delete one contact for one runtime callback.
     */
    @JvmName("deleteContact")
    internal fun deleteContact(
        id: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            contactBridge.deleteContact(runtimeHost, id)
        }
    }

    /**
     * List calendars for one runtime callback.
     */
    @JvmName("listCalendars")
    internal fun listCalendars(): RuntimeHostCalendarListResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostCalendarListResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            calendarBridge.listCalendars(runtimeHost)
        }
    }

    /**
     * List calendar events for one runtime callback.
     */
    @JvmName("listCalendarEvents")
    internal fun listCalendarEvents(
        query: RuntimeHostCalendarEventQuery,
    ): RuntimeHostCalendarEventListResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostCalendarEventListResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            calendarBridge.listCalendarEvents(runtimeHost, query)
        }
    }

    /**
     * Read one calendar event for one runtime callback.
     */
    @JvmName("readCalendarEvent")
    internal fun readCalendarEvent(
        id: String,
    ): RuntimeHostCalendarEventResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostCalendarEventResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            calendarBridge.readCalendarEvent(runtimeHost, id)
        }
    }

    /**
     * Create one calendar event for one runtime callback.
     */
    @JvmName("createCalendarEvent")
    internal fun createCalendarEvent(
        draft: RuntimeHostCalendarEventDraft,
    ): RuntimeHostCalendarEventCreateResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostCalendarEventCreateResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            calendarBridge.createCalendarEvent(runtimeHost, draft)
        }
    }

    /**
     * Update one calendar event for one runtime callback.
     */
    @JvmName("updateCalendarEvent")
    internal fun updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            calendarBridge.updateCalendarEvent(runtimeHost, id, draft)
        }
    }

    /**
     * Delete one calendar event for one runtime callback.
     */
    @JvmName("deleteCalendarEvent")
    internal fun deleteCalendarEvent(
        id: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            calendarBridge.deleteCalendarEvent(runtimeHost, id)
        }
    }

    /**
     * Return whether one outbound URL can be opened for one runtime callback.
     */
    @JvmName("canOpenUrl")
    internal fun canOpenUrl(
        url: String,
    ): Boolean {
        return withRuntimeHost(
            onMissing = { false },
        ) { runtimeHost ->
            intentBridge.canOpenUrl(runtimeHost, url)
        }
    }

    /**
     * Open one outbound URL for one runtime callback.
     */
    @JvmName("openUrl")
    internal fun openUrl(
        url: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            intentBridge.openUrl(runtimeHost, url)
        }
    }

    /**
     * Open one outbound path for one runtime callback.
     */
    @JvmName("openPath")
    internal fun openPath(
        path: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            intentBridge.openPath(runtimeHost, path)
        }
    }

    /**
     * Share one outbound text payload for one runtime callback.
     */
    @JvmName("shareText")
    internal fun shareText(
        text: String,
        contentType: String?,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            intentBridge.shareText(runtimeHost, text, contentType)
        }
    }

    /**
     * Read whether Android location services are enabled for one runtime callback.
     */
    @JvmName("locationServicesEnabled")
    internal fun locationServicesEnabled(): RuntimeHostLocationServicesResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostLocationServicesResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            locationBridge.locationServicesEnabled(runtimeHost)
        }
    }

    /**
     * Read one last-known Android location sample for one runtime callback.
     */
    @JvmName("locationLastKnown")
    internal fun locationLastKnown(): RuntimeHostLocationLastKnownResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostLocationLastKnownResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            locationBridge.locationLastKnown(runtimeHost)
        }
    }

    /**
     * Open one Android location watch for one runtime callback.
     */
    @JvmName("locationWatchOpen")
    internal fun locationWatchOpen(
        watchId: String,
        accuracy: Int,
        minimumIntervalNs: Long,
        minimumDistanceMeters: Double,
        includeHeading: Boolean,
    ): Int {
        val decodedAccuracy = when (accuracy) {
            1 -> RuntimeHostLocationAccuracy.Passive
            2 -> RuntimeHostLocationAccuracy.Low
            3 -> RuntimeHostLocationAccuracy.Balanced
            4 -> RuntimeHostLocationAccuracy.High
            5 -> RuntimeHostLocationAccuracy.Best
            else -> return 2
        }

        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            locationBridge.locationWatchOpen(
                runtimeHost = runtimeHost,
                watchId = watchId,
                options = RuntimeHostLocationWatchOptions(
                    accuracy = decodedAccuracy,
                    minimumIntervalNs = minimumIntervalNs,
                    minimumDistanceMeters = minimumDistanceMeters,
                    includeHeading = includeHeading,
                ),
            )
        }
    }

    /**
     * Close one Android location watch for one runtime callback.
     */
    @JvmName("locationWatchClose")
    internal fun locationWatchClose(
        watchId: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            locationBridge.locationWatchClose(runtimeHost, watchId)
        }
    }

    /**
     * Post one notification for one runtime callback.
     */
    @JvmName("postNotification")
    internal fun postNotification(
        identifier: String,
        title: String,
        body: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            notificationBridge.postNotification(
                runtimeHost = runtimeHost,
                request = RuntimeHostNotificationRequest(
                    identifier = identifier,
                    title = title,
                    body = body,
                ),
            )
        }
    }

    /**
     * Cancel one notification for one runtime callback.
     */
    @JvmName("cancelNotification")
    internal fun cancelNotification(
        identifier: String,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            notificationBridge.cancelNotification(runtimeHost, identifier)
        }
    }

    /**
     * Cancel every notification for one runtime callback.
     */
    @JvmName("cancelAllNotifications")
    internal fun cancelAllNotifications(): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            notificationBridge.cancelAllNotifications(runtimeHost)
        }
    }

    /**
     * List one page of media for one runtime callback.
     */
    @JvmName("listMedia")
    internal fun listMedia(
        cursor: String?,
        hasLimit: Boolean,
        limit: Int,
        kinds: IntArray,
        includeHidden: Boolean,
    ): RuntimeHostMediaListResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostMediaListResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            mediaBridge.listMedia(
                runtimeHost = runtimeHost,
                cursor = cursor,
                hasLimit = hasLimit,
                limit = limit,
                kinds = kinds,
                includeHidden = includeHidden,
            )
        }
    }

    /**
     * Read one media descriptor for one runtime callback.
     */
    @JvmName("readMedia")
    internal fun readMedia(
        identifier: String,
    ): RuntimeHostMediaReadResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostMediaReadResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            mediaBridge.readMedia(runtimeHost, identifier)
        }
    }

    /**
     * Import one local path into the host media library for one runtime callback.
     */
    @JvmName("importMediaPath")
    internal fun importMediaPath(
        path: String,
        kind: Int,
    ): RuntimeHostMediaImportPathResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostMediaImportPathResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            mediaBridge.importMediaPath(runtimeHost, path, kind)
        }
    }

    /**
     * Delete one batch of media assets for one runtime callback.
     */
    @JvmName("deleteMedia")
    internal fun deleteMedia(
        identifiers: Array<String>,
    ): RuntimeHostMediaDeleteResponse {
        return withRuntimeHost(
            onMissing = { RuntimeHostMediaDeleteResponse(status = hostStatusNotFound) },
        ) { runtimeHost ->
            mediaBridge.deleteMedia(runtimeHost, identifiers)
        }
    }

    /**
     * Share one outbound file-path list for one runtime callback.
     */
    @JvmName("sharePaths")
    internal fun sharePaths(
        paths: Array<String>,
        contentType: String?,
    ): Int {
        return withRuntimeHost(
            onMissing = { hostStatusNotFound },
        ) { runtimeHost ->
            intentBridge.sharePaths(runtimeHost, paths.toList(), contentType)
        }
    }

}
