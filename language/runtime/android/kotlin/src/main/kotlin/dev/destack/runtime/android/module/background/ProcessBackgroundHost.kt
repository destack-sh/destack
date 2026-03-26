package dev.destack.runtime.android.module.background

import android.content.Context

private const val nanosecondsPerMillisecond: Long = 1_000_000L

/**
 * The Android process-level background request surface.
 */
public class ProcessBackgroundHost private constructor(
    private val coordinator: BackgroundCoordinator,
) : BackgroundRequests {
    /**
     * Detach the live event sink from this request surface.
     */
    public fun detach() {
        coordinator.detachEvents()
    }

    /**
     * Read the Android background scheduler status.
     */
    override fun backgroundStatus(): RuntimeHostBackgroundStatusResponse {
        return coordinator.backgroundStatus()
    }

    /**
     * List registered Android background tasks.
     */
    override fun listBackgroundTasks(): RuntimeHostBackgroundTaskListResponse {
        return coordinator.listBackgroundTasks()
    }

    /**
     * Register one Android background task.
     */
    override fun registerBackgroundTask(
        options: RuntimeHostBackgroundTaskOptions,
    ): Int {
        return coordinator.registerBackgroundTask(options)
    }

    /**
     * Unregister one Android background task.
     */
    override fun unregisterBackgroundTask(
        identifier: String,
    ): Int {
        return coordinator.unregisterBackgroundTask(identifier)
    }

    /**
     * Trigger one Android background task for testing.
     */
    override fun triggerBackgroundTask(
        identifier: String,
    ): RuntimeHostBackgroundTriggerResponse {
        return coordinator.triggerBackgroundTask(identifier)
    }

    /**
     * Complete one Android background task execution.
     */
    override fun completeBackgroundTask(
        executionId: String,
        result: RuntimeHostBackgroundTaskResult,
    ): Int {
        return coordinator.completeBackgroundTask(executionId, result)
    }

    public companion object {
        /**
         * Attach one process-level background surface for the given context and event sink.
         */
        public fun attach(
            context: Context?,
            events: BackgroundEvents,
        ): BackgroundRequests {
            // return the explicit unsupported surface when no context exists
            val context = context ?: return UnsupportedBackgroundRequests

            // reuse the process coordinator for the application package
            val coordinator = ProcessBackgroundHostRegistry.coordinator(context)

            // attach the live event sink before returning the request surface
            coordinator.attachEvents(events)

            return ProcessBackgroundHost(coordinator)
        }
    }
}

/**
 * The process-local registry for Android background coordinators.
 */
internal object ProcessBackgroundHostRegistry {
    /**
     * The coordinators keyed by application package name.
     */
    private val coordinators: MutableMap<String, BackgroundCoordinator> = mutableMapOf()

    /**
     * Return the process coordinator for the application context.
     */
    fun coordinator(
        context: Context,
    ): BackgroundCoordinator {
        // derive the stable key from the application context
        val applicationContext = context.applicationContext
        val packageName = applicationContext.packageName

        // create the process coordinator once per application package
        return synchronized(coordinators) {
            coordinators.getOrPut(packageName) {
                // build the shared registration store first
                val registrations = BackgroundRegistrationStore(
                    applicationContext.getSharedPreferences(
                        backgroundPreferencesName,
                        Context.MODE_PRIVATE,
                    ),
                )

                BackgroundCoordinator(
                    scheduler = WorkManagerBackend(applicationContext),
                    registrations = registrations,
                    monotonicNowNs = System::nanoTime,
                    wallClockNowNs = {
                        System.currentTimeMillis() * nanosecondsPerMillisecond
                    },
                )
            }
        }
    }
}
