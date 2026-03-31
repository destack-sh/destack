package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise the intent bridge lane.
 */
class IntentBridgeTest {
    /**
     * Route one can-open-url request into the attached runtime host.
     */
    @Test
    fun testCanOpenUrlRoutesIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
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
        val isSupported = bridge.intentCanOpenUrl("https://example.com")

        assertEquals(true, isSupported)
        assertEquals(listOf("https://example.com"), intentRequests.canOpenUrlCalls)
    }

    /**
     * Send one intent event through the runtime ingress path.
     */
    @Test
    fun testSendIntentEventNotifiesRuntime() {
        val runtimeApi = RuntimeIngressSpy()
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
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

        bridge.notifyIntentEvent(event)

        assertEquals(listOf(sessionHandle to event), runtimeApi.intentEvents)
    }
}
