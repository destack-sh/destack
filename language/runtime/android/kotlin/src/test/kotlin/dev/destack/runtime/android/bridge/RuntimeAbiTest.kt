package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle

/**
 * The live Android bridge test helpers.
 */
internal object RuntimeAbiTest {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()

        val explicitLibraryPath = System.getenv("DESTACK_RUNTIME_HOST_ANDROID_TESTING_LIBRARY")
        if (!explicitLibraryPath.isNullOrBlank()) {
            System.load(explicitLibraryPath)
        } else {
            System.loadLibrary("destack_runtime_host_android_testing")
        }
    }

    /**
     * Open one live runtime session for bridge tests.
     */
    fun openTestSession(): HostSessionHandle {
        return HostSessionHandle(rawValue = nativeOpenTestSession())
    }

    /**
     * Close one live runtime session for bridge tests.
     */
    fun closeTestSession(
        sessionHandle: HostSessionHandle,
    ) {
        nativeCloseTestSession(sessionHandle.rawValue)
    }

    /**
     * Submit one live document request for bridge tests.
     */
    fun submitTestDocumentRequest(
        sessionHandle: HostSessionHandle,
        requestId: Long,
        mimeTypes: Array<String>,
        extensions: Array<String>,
        allowsMultipleSelection: Boolean,
        allowsDirectorySelection: Boolean,
        copiesToSandbox: Boolean,
    ): Int {
        return nativeSubmitTestDocumentRequest(
            sessionHandle.rawValue,
            requestId,
            mimeTypes,
            extensions,
            allowsMultipleSelection,
            allowsDirectorySelection,
            copiesToSandbox,
        )
    }

    /**
     * Submit one live permission request for bridge tests.
     */
    fun submitTestPermissionRequest(
        sessionHandle: HostSessionHandle,
        requestId: Long,
        permissions: Array<String>,
    ): Int {
        return nativeSubmitTestPermissionRequest(
            sessionHandle.rawValue,
            requestId,
            permissions,
        )
    }

    /**
     * Open permission settings through the live bridge test path.
     */
    fun openTestPermissionSettings(
        sessionHandle: HostSessionHandle,
    ): Int {
        return nativeOpenTestPermissionSettings(sessionHandle.rawValue)
    }

    /**
     * Query can-open-url through the live bridge test path.
     */
    fun testIntentCanOpenUrl(
        sessionHandle: HostSessionHandle,
        url: String,
    ): Pair<Int, Boolean> {
        val values = nativeTestIntentCanOpenUrl(sessionHandle.rawValue, url)

        return values[0].toInt() to (values[1] != 0L)
    }

    /**
     * Query location-services-enabled through the live bridge test path.
     */
    fun testLocationServicesEnabled(
        sessionHandle: HostSessionHandle,
    ): Pair<Int, Boolean> {
        val values = nativeTestLocationServicesEnabled(sessionHandle.rawValue)

        return values[0].toInt() to (values[1] != 0L)
    }

    /**
     * Open one location watch through the live bridge test path.
     */
    fun testLocationWatchOpen(
        sessionHandle: HostSessionHandle,
        watchId: String,
        accuracy: Int,
        minimumIntervalNs: Long,
        minimumDistanceMeters: Double,
        includeHeading: Boolean,
    ): Int {
        return nativeTestLocationWatchOpen(
            sessionHandle.rawValue,
            watchId,
            accuracy,
            minimumIntervalNs,
            minimumDistanceMeters,
            includeHeading,
        )
    }

    /**
     * Close one location watch through the live bridge test path.
     */
    fun testLocationWatchClose(
        sessionHandle: HostSessionHandle,
        watchId: String,
    ): Int {
        return nativeTestLocationWatchClose(
            sessionHandle.rawValue,
            watchId,
        )
    }

    /**
     * Submit one live notification request through the bridge test path.
     */
    fun submitTestNotificationPost(
        sessionHandle: HostSessionHandle,
        identifier: String,
        title: String,
        body: String,
    ): Int {
        return nativeSubmitTestNotificationPost(
            sessionHandle.rawValue,
            identifier,
            title,
            body,
        )
    }

    private external fun nativeOpenTestSession(): Long

    private external fun nativeCloseTestSession(
        sessionHandle: Long,
    )

    private external fun nativeSubmitTestDocumentRequest(
        sessionHandle: Long,
        requestId: Long,
        mimeTypes: Array<String>,
        extensions: Array<String>,
        allowsMultipleSelection: Boolean,
        allowsDirectorySelection: Boolean,
        copiesToSandbox: Boolean,
    ): Int

    private external fun nativeSubmitTestPermissionRequest(
        sessionHandle: Long,
        requestId: Long,
        permissions: Array<String>,
    ): Int

    private external fun nativeOpenTestPermissionSettings(
        sessionHandle: Long,
    ): Int

    private external fun nativeTestIntentCanOpenUrl(
        sessionHandle: Long,
        url: String,
    ): LongArray

    private external fun nativeTestLocationServicesEnabled(
        sessionHandle: Long,
    ): LongArray

    private external fun nativeTestLocationWatchOpen(
        sessionHandle: Long,
        watchId: String,
        accuracy: Int,
        minimumIntervalNs: Long,
        minimumDistanceMeters: Double,
        includeHeading: Boolean,
    ): Int

    private external fun nativeTestLocationWatchClose(
        sessionHandle: Long,
        watchId: String,
    ): Int

    private external fun nativeSubmitTestNotificationPost(
        sessionHandle: Long,
        identifier: String,
        title: String,
        body: String,
    ): Int
}
