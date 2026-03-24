package dev.destack.runtime.android

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create one permission request and one permission result event.
 */
class RuntimeHostPermissionTest {
    @Test
    fun testCreatePermissionRequestAndEvent() {
        val request = RuntimeHostPermissionRequest(
            requestId = HostRequestId(rawValue = 6),
            permission = "location",
        )
        val event = RuntimeHostPermissionEvent(
            requestId = HostRequestId(rawValue = 6),
            permission = "location",
            isGranted = true,
        )

        assertEquals(HostRequestId(rawValue = 6), request.requestId)
        assertEquals("location", request.permission)
        assertEquals(HostRequestId(rawValue = 6), event.requestId)
        assertEquals("location", event.permission)
        assertEquals(true, event.isGranted)
    }
}
