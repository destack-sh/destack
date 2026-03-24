package dev.destack.runtime.android

import androidx.lifecycle.Lifecycle

import dev.destack.runtime.android.embedder.ActivityObserver
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleState

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Forward Android lifecycle-observer callbacks into the attached runtime host.
 */
class ActivityObserverTest {
    @Test
    fun testActivityLifecycleObserverForwardsExpectedStates() {
        val lifecycleEvents = RecordingLifecycleSink()
        val runtimeHost = createRuntimeHost(
            lifecycleEvents = lifecycleEvents,
        )
        val observer = ActivityObserver(runtimeHost)
        val owner = TestLifecycleOwner()

        observer.onCreate(owner)
        observer.onStart(owner)
        observer.onResume(owner)
        observer.onPause(owner)
        observer.onStop(owner)
        observer.onDestroy(owner)

        assertEquals(
            listOf(
                activityLifecycleEvent(RuntimeHostLifecycleState.Initializing),
                activityLifecycleEvent(RuntimeHostLifecycleState.Running),
                activityLifecycleEvent(RuntimeHostLifecycleState.Running),
                activityLifecycleEvent(RuntimeHostLifecycleState.Paused),
                activityLifecycleEvent(RuntimeHostLifecycleState.Stopped),
                activityLifecycleEvent(RuntimeHostLifecycleState.Destroyed),
            ),
            lifecycleEvents.events,
        )
    }
}
