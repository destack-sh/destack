package dev.destack.runtime.android

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.permission.RuntimeHostPermission
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
            permission = RuntimeHostPermission.Location,
        )
        val event = RuntimeHostPermissionEvent(
            requestId = HostRequestId(rawValue = 6),
            permission = RuntimeHostPermission.Location,
            isGranted = true,
        )

        assertEquals(HostRequestId(rawValue = 6), request.requestId)
        assertEquals(RuntimeHostPermission.Location, request.permission)
        assertEquals(HostRequestId(rawValue = 6), event.requestId)
        assertEquals(RuntimeHostPermission.Location, event.permission)
        assertEquals(true, event.isGranted)
    }
}
