package dev.destack.runtime.android.module.lifecycle

/**
 * The normalized lifecycle state delivered from Android into one runtime session.
 */
public enum class RuntimeHostLifecycleState {
    /**
     * The runtime host has not finished start-up yet.
     */
    Initializing,

    /**
     * The runtime host is active and may process host interaction.
     */
    Running,

    /**
     * The runtime host is paused by the Android shell.
     */
    Paused,

    /**
     * The runtime host is stopped by the Android shell.
     */
    Stopped,

    /**
     * The runtime host is being destroyed by the Android shell.
     */
    Destroyed,
}

/**
 * The lifecycle source embedder that originated one lifecycle event.
 */
public enum class RuntimeHostLifecycleSourceKind {
    /**
     * The event came from one application embedder.
     */
    Application,

    /**
     * The event came from one activity embedder.
     */
    Activity,
}

/**
 * One normalized lifecycle ingress event delivered into one runtime session.
 */
public data class RuntimeHostLifecycleEvent(
    /**
     * The lifecycle source embedder that originated this event.
     */
    val sourceKind: RuntimeHostLifecycleSourceKind,

    /**
     * The next normalized lifecycle state for the runtime session.
     */
    val state: RuntimeHostLifecycleState,
)
