package dev.destack.runtime.android.module.background

import android.content.Context

import androidx.concurrent.futures.CallbackToFutureAdapter
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.Data
import androidx.work.ExistingWorkPolicy
import androidx.work.ListenableWorker
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import androidx.work.WorkerParameters

import com.google.common.util.concurrent.ListenableFuture

import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusOk

import java.util.concurrent.TimeUnit

private const val nanosecondsPerMillisecond: Long = 1_000_000L
private const val backgroundRetryBackoffSeconds: Long = 10L
private const val backgroundWorkNamePrefix: String = "dev.destack.runtime.background."

internal const val backgroundWorkIdentifierKey: String = "identifier"

/**
 * The Android scheduler backend used by the process-level background host.
 */
internal interface BackgroundSchedulerBackend {
    /**
     * Return the scheduler availability for this process.
     */
    fun schedulerStatus(): RuntimeHostBackgroundStatus?

    /**
     * Schedule one task record for one future execution time.
     */
    fun schedule(
        record: BackgroundTaskRecord,
        runAtUnixNs: Long,
    ): Int

    /**
     * Cancel one scheduled task by stable identifier.
     */
    fun cancel(
        identifier: String,
    ): Int
}

/**
 * The Android `WorkManager` backend.
 */
internal class WorkManagerBackend(
    private val context: Context,
) : BackgroundSchedulerBackend {
    /**
     * The shared WorkManager instance for this process.
     */
    private val workManager: WorkManager = WorkManager.getInstance(context)

    override fun schedulerStatus(): RuntimeHostBackgroundStatus? {
        // WorkManager is available when the runtime host can resolve its process instance
        return RuntimeHostBackgroundStatus.Available
    }

    override fun schedule(
        record: BackgroundTaskRecord,
        runAtUnixNs: Long,
    ): Int {
        val descriptor = record.descriptor

        // compute the initial delay from the target time
        val delayMs = ((runAtUnixNs - System.currentTimeMillis() * nanosecondsPerMillisecond)
            .coerceAtLeast(0L) / nanosecondsPerMillisecond)
            .coerceAtLeast(1L)

        // build the work constraints from the runtime descriptor
        val constraints = workConstraints(descriptor)

        // encode the runtime identifier into the work payload
        val inputData = Data.Builder()
            .putString(backgroundWorkIdentifierKey, descriptor.identifier)
            .build()

        // build one one-shot work request for the next eligible execution
        val requestBuilder = OneTimeWorkRequestBuilder<RuntimeHostBackgroundWorker>()
            .setInputData(inputData)
            .setConstraints(constraints)
            .setInitialDelay(delayMs, TimeUnit.MILLISECONDS)
            .setBackoffCriteria(
                BackoffPolicy.LINEAR,
                backgroundRetryBackoffSeconds,
                TimeUnit.SECONDS,
            )

        // keep app refresh launches lightweight and opportunistic
        if (usesExpeditedWork(descriptor.trigger)) {
            requestBuilder.setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
        }

        // finalize the unique work request for this identifier
        val request = requestBuilder
            .addTag(backgroundWorkName(descriptor.identifier))
            .build()

        // apply the requested conflict policy for the stable identifier
        workManager.enqueueUniqueWork(
            backgroundWorkName(descriptor.identifier),
            when (descriptor.conflictPolicy) {
                RuntimeHostBackgroundConflictPolicy.Replace -> ExistingWorkPolicy.REPLACE
                RuntimeHostBackgroundConflictPolicy.Keep -> ExistingWorkPolicy.KEEP
            },
            request,
        )

        return hostStatusOk
    }

    override fun cancel(
        identifier: String,
    ): Int {
        // cancel the unique work for the stable identifier
        workManager.cancelUniqueWork(backgroundWorkName(identifier))

        return hostStatusOk
    }
}

/**
 * Return whether one trigger class should use the expedited work path.
 */
internal fun usesExpeditedWork(
    trigger: RuntimeHostBackgroundTriggerKind,
): Boolean {
    return trigger == RuntimeHostBackgroundTriggerKind.AppRefresh
}

/**
 * Build the effective WorkManager constraints for one runtime descriptor.
 */
private fun workConstraints(
    descriptor: RuntimeHostBackgroundTaskDescriptor,
): Constraints {
    return Constraints.Builder()
        .setRequiredNetworkType(
            when (descriptor.network) {
                RuntimeHostBackgroundNetworkRequirement.None -> NetworkType.NOT_REQUIRED
                RuntimeHostBackgroundNetworkRequirement.Connected -> NetworkType.CONNECTED
                RuntimeHostBackgroundNetworkRequirement.Unmetered -> NetworkType.UNMETERED
            },
        )
        .setRequiresCharging(descriptor.requiresCharging)
        .setRequiresDeviceIdle(descriptor.requiresIdle)
        .build()
}

/**
 * The `WorkManager` worker that forwards launches into the background coordinator.
 */
public class RuntimeHostBackgroundWorker(
    appContext: Context,
    workerParams: WorkerParameters,
) : ListenableWorker(appContext, workerParams) {
    /**
     * The active execution identifier while this worker is running.
     */
    @Volatile
    private var executionId: String? = null

    override fun startWork(): ListenableFuture<Result> {
        return CallbackToFutureAdapter.getFuture { completer ->
            // require one stable runtime task identifier
            val identifier = inputData.getString(backgroundWorkIdentifierKey)
            if (identifier.isNullOrBlank()) {
                completer.set(Result.failure())

                return@getFuture "RuntimeHostBackgroundWorker-missing-identifier"
            }

            // start one runtime-owned background execution
            val startedExecutionId = ProcessBackgroundHostRegistry
                .coordinator(applicationContext)
                .startExecution(identifier) { result ->
                    completer.set(
                        when (result) {
                            RuntimeHostBackgroundTaskResult.Success -> Result.success()
                            RuntimeHostBackgroundTaskResult.Retry -> Result.retry()
                            RuntimeHostBackgroundTaskResult.Failure -> Result.failure()
                        },
                    )
                }

            // fail immediately when the runtime no longer owns this registration
            if (startedExecutionId == null) {
                completer.set(Result.failure())

                return@getFuture "RuntimeHostBackgroundWorker-missing-registration"
            }

            // retain the execution identifier for stop notifications
            executionId = startedExecutionId

            "RuntimeHostBackgroundWorker-$startedExecutionId"
        }
    }

    override fun onStopped() {
        // forward worker expiration into the process coordinator
        executionId?.let { executionId ->
            ProcessBackgroundHostRegistry
                .coordinator(applicationContext)
                .stopExecution(executionId)
        }
    }
}

/**
 * Build the unique work name for one task identifier.
 */
private fun backgroundWorkName(
    identifier: String,
): String {
    return backgroundWorkNamePrefix + identifier
}
