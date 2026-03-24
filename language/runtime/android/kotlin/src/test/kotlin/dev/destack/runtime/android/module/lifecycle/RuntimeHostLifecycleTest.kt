package dev.destack.runtime.android

import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleEvent
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleSourceKind
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleState

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create one normalized Android lifecycle event.
 */
class RuntimeHostLifecycleTest {
    @Test
    fun testCreateLifecycleEvent() {
        val event = RuntimeHostLifecycleEvent(
            sourceKind = RuntimeHostLifecycleSourceKind.Activity,
            state = RuntimeHostLifecycleState.Running,
        )

        assertEquals(RuntimeHostLifecycleSourceKind.Activity, event.sourceKind)
        assertEquals(RuntimeHostLifecycleState.Running, event.state)
    }
}
