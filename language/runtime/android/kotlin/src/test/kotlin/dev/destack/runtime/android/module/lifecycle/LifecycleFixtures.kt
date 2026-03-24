package dev.destack.runtime.android

import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleEvent
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleSourceKind
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleState

/**
 * One recording lifecycle sink for Android host tests.
 */
internal class RecordingLifecycleSink : LifecycleEvents {
    val events: MutableList<RuntimeHostLifecycleEvent> = mutableListOf()

    override fun sendLifecycleEvent(
        event: RuntimeHostLifecycleEvent,
    ) {
        events += event
    }
}

/**
 * Build one application lifecycle event fixture.
 */
internal fun applicationLifecycleEvent(
    state: RuntimeHostLifecycleState,
): RuntimeHostLifecycleEvent {
    return RuntimeHostLifecycleEvent(
        sourceKind = RuntimeHostLifecycleSourceKind.Application,
        state = state,
    )
}

/**
 * Build one activity lifecycle event fixture.
 */
internal fun activityLifecycleEvent(
    state: RuntimeHostLifecycleState,
): RuntimeHostLifecycleEvent {
    return RuntimeHostLifecycleEvent(
        sourceKind = RuntimeHostLifecycleSourceKind.Activity,
        state = state,
    )
}
