package dev.destack.runtime.android.module.background

import dev.destack.runtime.android.core.hostStatusNotSupported

/**
 * One Android background status response.
 */
public data class RuntimeHostBackgroundStatusResponse(
    /**
     * The bridge status code for this request.
     */
    val status: Int,

    /**
     * The reported scheduler availability when available.
     */
    val schedulerStatus: RuntimeHostBackgroundStatus? = null,
)

/**
 * One Android background task-list response.
 */
public data class RuntimeHostBackgroundTaskListResponse(
    /**
     * The bridge status code for this request.
     */
    val status: Int,

    /**
     * The returned task descriptors.
     */
    val descriptors: List<RuntimeHostBackgroundTaskDescriptor> = emptyList(),
)

/**
 * One Android background trigger response.
 */
public data class RuntimeHostBackgroundTriggerResponse(
    /**
     * The bridge status code for this request.
     */
    val status: Int,

    /**
     * Whether the host triggered one task execution.
     */
    val isTriggered: Boolean = false,
)

/**
 * The background request surface attached to one Android runtime host.
 */
public interface BackgroundRequests {
    /**
     * Read the Android background scheduler status.
     */
    public fun backgroundStatus(): RuntimeHostBackgroundStatusResponse

    /**
     * List registered Android background tasks.
     */
    public fun listBackgroundTasks(): RuntimeHostBackgroundTaskListResponse

    /**
     * Register one Android background task.
     */
    public fun registerBackgroundTask(
        options: RuntimeHostBackgroundTaskOptions,
    ): Int

    /**
     * Unregister one Android background task.
     */
    public fun unregisterBackgroundTask(
        identifier: String,
    ): Int

    /**
     * Trigger one Android background task for testing.
     */
    public fun triggerBackgroundTask(
        identifier: String,
    ): RuntimeHostBackgroundTriggerResponse

    /**
     * Complete one Android background task execution.
     */
    public fun completeBackgroundTask(
        executionId: String,
        result: RuntimeHostBackgroundTaskResult,
    ): Int
}

/**
 * The explicit unsupported background request surface for one Android runtime host.
 */
public object UnsupportedBackgroundRequests : BackgroundRequests {
    /**
     * Read the Android background scheduler status.
     */
    override fun backgroundStatus(): RuntimeHostBackgroundStatusResponse {
        return RuntimeHostBackgroundStatusResponse(status = hostStatusNotSupported)
    }

    /**
     * List registered Android background tasks.
     */
    override fun listBackgroundTasks(): RuntimeHostBackgroundTaskListResponse {
        return RuntimeHostBackgroundTaskListResponse(status = hostStatusNotSupported)
    }

    /**
     * Register one Android background task.
     */
    override fun registerBackgroundTask(
        options: RuntimeHostBackgroundTaskOptions,
    ): Int {
        return hostStatusNotSupported
    }

    /**
     * Unregister one Android background task.
     */
    override fun unregisterBackgroundTask(
        identifier: String,
    ): Int {
        return hostStatusNotSupported
    }

    /**
     * Trigger one Android background task for testing.
     */
    override fun triggerBackgroundTask(
        identifier: String,
    ): RuntimeHostBackgroundTriggerResponse {
        return RuntimeHostBackgroundTriggerResponse(status = hostStatusNotSupported)
    }

    /**
     * Complete one Android background task execution.
     */
    override fun completeBackgroundTask(
        executionId: String,
        result: RuntimeHostBackgroundTaskResult,
    ): Int {
        return hostStatusNotSupported
    }
}
