package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent

import org.junit.Assert.assertEquals
import org.junit.Assume.assumeTrue
import org.junit.Test

/**
 * Exercise the intent bridge lane.
 */
class IntentBridgeTest {
    /**
     * Roundtrip one live can-open-url request through the process runtime ABI.
     */
    @Test
    fun testProcessRuntimeAbiRoundtripsIntentCanOpenUrlRequest() {
        assumeTrue("missing DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY", hasRuntimeBridgeLibrary())

        val sessionHandle = RuntimeAbiTest.openTestSession()

        try {
            val bridge = RuntimeBridge(sessionHandle, ProcessRuntimeAbi)
            val permissionRequests = PermissionRequestRecorder()
            val documentRequests = DocumentRequestRecorder()
            val intentRequests = IntentRequestRecorder().also {
                it.isOpenUrlSupported = true
            }
            val runtimeHost = createBridgeRuntimeHost(
                sessionHandle = sessionHandle,
                permissionRequests = permissionRequests,
                documentRequests = documentRequests,
                intentRequests = intentRequests,
            )

            try {
                bridge.attach(runtimeHost)

                val (status, isSupported) = RuntimeAbiTest.testIntentCanOpenUrl(
                    sessionHandle = sessionHandle,
                    url = "https://example.com",
                )

                assertEquals(0, status)
                assertEquals(true, isSupported)
                assertEquals(listOf("https://example.com"), intentRequests.canOpenUrlCalls)
            } finally {
                bridge.detach()
            }
        } finally {
            RuntimeAbiTest.closeTestSession(sessionHandle)
        }
    }

    /**
     * Route one can-open-url request into the attached runtime host.
     */
    @Test
    fun testCanOpenUrlRoutesIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val intentRequests = IntentRequestRecorder().also {
            it.isOpenUrlSupported = true
        }
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = intentRequests,
        )

        bridge.attach(runtimeHost)
        val isSupported = bridge.canOpenUrl("https://example.com")

        assertEquals(true, isSupported)
        assertEquals(listOf("https://example.com"), intentRequests.canOpenUrlCalls)
    }

    /**
     * Send one intent event through the runtime ingress path.
     */
    @Test
    fun testSendIntentEventNotifiesRuntime() {
        val bindings = RuntimeAbiSpy()
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        bridge.attach(runtimeHost)

        val event = RuntimeHostIntentEvent(
            source = "app://origin",
            payload = dev.destack.runtime.android.module.intent.RuntimeHostIntentPayload.ShareText(
                text = "hello",
                contentType = "text/plain",
            ),
        )

        bridge.sendIntentEvent(event)

        assertEquals(listOf(sessionHandle to event), bindings.intentEvents)
    }
}
