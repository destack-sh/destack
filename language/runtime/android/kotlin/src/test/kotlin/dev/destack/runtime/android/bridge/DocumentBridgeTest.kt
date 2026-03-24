package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult

import org.junit.Assert.assertEquals
import org.junit.Assume.assumeTrue
import org.junit.Test

/**
 * Exercise the document bridge lane.
 */
class DocumentBridgeTest {
    /**
     * Roundtrip one live document request and one completion through the process runtime ABI.
     */
    @Test
    fun testProcessRuntimeAbiRoundtripsDocumentRequest() {
        assumeTrue("missing DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY", hasRuntimeBridgeLibrary())

        val sessionHandle = RuntimeAbiTest.openTestSession()

        try {
            val bridge = RuntimeBridge(
                sessionHandle = sessionHandle,
                bindings = ProcessRuntimeAbi,
            )
            val permissionRequests = PermissionRequestRecorder()
            val documentRequests = DocumentRequestRecorder()
            val runtimeHost = createBridgeRuntimeHost(
                sessionHandle = sessionHandle,
                permissionRequests = permissionRequests,
                documentRequests = documentRequests,
                intentRequests = IntentRequestRecorder(),
            )
            val expectedRequest = RuntimeHostDocumentRequest(
                requestId = HostRequestId(rawValue = 3),
                contentTypes = listOf("image/png"),
                allowsMultipleSelection = true,
            )
            try {
                bridge.attach(runtimeHost)

                val submitStatus = RuntimeAbiTest.submitTestDocumentRequest(
                    sessionHandle,
                    requestId = 3,
                    mimeTypes = arrayOf("image/png"),
                    extensions = emptyArray(),
                    allowsMultipleSelection = true,
                    allowsDirectorySelection = false,
                    copiesToSandbox = false,
                )

                assertEquals(0, submitStatus)
                assertEquals(listOf(expectedRequest), documentRequests.requests)
            } finally {
                bridge.detach()
            }
        } finally {
            RuntimeAbiTest.closeTestSession(sessionHandle)
        }
    }

    /**
     * Route one document request from one runtime payload into the attached runtime host.
     */
    @Test
    fun testSubmitDocumentRequestRoutesIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val documentRequests = DocumentRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = documentRequests,
            intentRequests = IntentRequestRecorder(),
        )
        bridge.attach(runtimeHost)
        val status = bridge.submitDocumentRequest(
            requestId = 3,
            mimeTypes = arrayOf("image/png"),
            extensions = emptyArray(),
            allowsMultipleSelection = true,
            allowsDirectorySelection = false,
            copiesToSandbox = false,
        )

        assertEquals(0, status)
        assertEquals(
            listOf(
                RuntimeHostDocumentRequest(
                    requestId = HostRequestId(rawValue = 3),
                    contentTypes = listOf("image/png"),
                    allowsMultipleSelection = true,
                ),
            ),
            documentRequests.requests,
        )
    }

    /**
     * Send one document result through the runtime ingress path.
     */
    @Test
    fun testSendDocumentResultNotifiesRuntime() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        val result = RuntimeHostDocumentResult(
            requestId = HostRequestId(rawValue = 13),
            documents = listOf(
                RuntimeHostDocumentDescriptor(
                    uri = "file:///tmp/example.png",
                    displayName = "example.png",
                    contentType = "image/png",
                ),
            ),
        )

        bridge.attach(runtimeHost)
        bridge.sendDocumentResult(result)

        assertEquals(listOf(sessionHandle to result), bindings.documentResults)
    }

    /**
     * Report one missing runtime host after detach.
     */
    @Test
    fun testSubmitDocumentRequestReportsMissingRuntimeHostAfterDetach() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        bridge.attach(runtimeHost)
        bridge.detach()
        val status = bridge.submitDocumentRequest(
            requestId = 3,
            mimeTypes = arrayOf("image/png"),
            extensions = emptyArray(),
            allowsMultipleSelection = false,
            allowsDirectorySelection = false,
            copiesToSandbox = false,
        )

        assertEquals(3, status)
    }
}
