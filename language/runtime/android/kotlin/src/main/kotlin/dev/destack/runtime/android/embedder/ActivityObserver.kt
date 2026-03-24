package dev.destack.runtime.android.embedder

import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner

import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleEvent
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleSourceKind
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleState

/**
 * The lifecycle observer that forwards one Android activity embedder into one runtime host.
 */
internal class ActivityObserver(
    /**
     * The Android runtime host receiving lifecycle transitions.
     */
    public val runtimeHost: RuntimeHost,
) : DefaultLifecycleObserver {
    /**
     * Forward one activity create callback into the runtime host.
     */
    override fun onCreate(
        owner: LifecycleOwner,
    ) {
        sendLifecycleState(RuntimeHostLifecycleState.Initializing)
    }

    /**
     * Forward one activity start callback into the runtime host.
     */
    override fun onStart(
        owner: LifecycleOwner,
    ) {
        sendLifecycleState(RuntimeHostLifecycleState.Running)
    }

    /**
     * Forward one activity resume callback into the runtime host.
     */
    override fun onResume(
        owner: LifecycleOwner,
    ) {
        sendLifecycleState(RuntimeHostLifecycleState.Running)
    }

    /**
     * Forward one activity pause callback into the runtime host.
     */
    override fun onPause(
        owner: LifecycleOwner,
    ) {
        sendLifecycleState(RuntimeHostLifecycleState.Paused)
    }

    /**
     * Forward one activity stop callback into the runtime host.
     */
    override fun onStop(
        owner: LifecycleOwner,
    ) {
        sendLifecycleState(RuntimeHostLifecycleState.Stopped)
    }

    /**
     * Forward one activity destroy callback into the runtime host.
     */
    override fun onDestroy(
        owner: LifecycleOwner,
    ) {
        sendLifecycleState(RuntimeHostLifecycleState.Destroyed)
    }

    /**
     * Send one activity lifecycle state through the attached runtime host.
     */
    private fun sendLifecycleState(
        state: RuntimeHostLifecycleState,
    ) {
        runtimeHost.lifecycleEvents.sendLifecycleEvent(
            RuntimeHostLifecycleEvent(
                sourceKind = RuntimeHostLifecycleSourceKind.Activity,
                state = state,
            ),
        )
    }
}
