package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise bridge attachment and registration behavior.
 */
class RuntimeBridgeAttachTest {
    /**
     * Register the document, permission, background, calendar, contact, intent, location, media, and notification lanes for one attached runtime host.
     */
    @Test
    fun testAttachRegistersBridge() {
        val bindings = RuntimeAbiSpy()
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = ContactRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        bridge.attach(runtimeHost)

        assertEquals(listOf(sessionHandle), bindings.attachedSessionHandles)
    }

    /**
     * Reattach one bridge after detach on the same runtime session.
     */
    @Test
    fun testDetachUnregistersBridgeForReattach() {
        val bindings = RuntimeAbiSpy()
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = ContactRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        bridge.attach(runtimeHost)
        bridge.detach()
        bridge.attach(runtimeHost)

        assertEquals(listOf(sessionHandle, sessionHandle), bindings.attachedSessionHandles)
        assertEquals(listOf(sessionHandle), bindings.detachedSessionHandles)
    }

    /**
     * Reject one mismatched runtime host session handle.
     */
    @Test
    fun testAttachRejectsMismatchedRuntimeHostSessionHandle() {
        val bridge = RuntimeBridge(HostSessionHandle(rawValue = 7), RuntimeAbiSpy())
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = HostSessionHandle(rawValue = 9),
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = ContactRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        val error = org.junit.Assert.assertThrows(IllegalArgumentException::class.java) {
            bridge.attach(runtimeHost)
        }

        assertEquals(
            "runtime bridge session handle does not match the attached runtime host",
            error.message,
        )
    }

    /**
     * Leave one failed attach without one detach callback when registration never succeeded.
     */
    @Test
    fun testAttachDetachesBridgeAfterFailedRegistration() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy().also {
            it.attachStatus = 6
        }
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = ContactRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        val error = org.junit.Assert.assertThrows(IllegalStateException::class.java) {
            bridge.attach(runtimeHost)
        }

        assertEquals(
            "runtime bridge could not attach runtime abi bindings: 6",
            error.message,
        )
        assertEquals(listOf(sessionHandle), bindings.attachedSessionHandles)
        assertEquals(emptyList<HostSessionHandle>(), bindings.detachedSessionHandles)
    }

    /**
     * Reject one bridge attachment when the runtime ABI seam cannot attach.
     */
    @Test
    fun testAttachRejectsRuntimeAbiAttachFailure() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy().also {
            it.attachStatus = 6
        }
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = ContactRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        val error = org.junit.Assert.assertThrows(IllegalStateException::class.java) {
            bridge.attach(runtimeHost)
        }

        assertEquals(
            "runtime bridge could not attach runtime abi bindings: 6",
            error.message,
        )
        assertEquals(listOf(sessionHandle), bindings.attachedSessionHandles)
        assertEquals(emptyList<HostSessionHandle>(), bindings.detachedSessionHandles)
    }
}
