package dev.destack.runtime.android.bridge

import android.os.Handler
import android.os.Looper

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.background.BackgroundEvents
import dev.destack.runtime.android.module.notification.NotificationEvents

import java.util.concurrent.CountDownLatch

/**
 * One runtime-backed bridge for one attached Android runtime host.
 */
public class RuntimeBridge internal constructor(
    /**
     * The runtime session routed through this bridge.
     */
    sessionHandle: HostSessionHandle,
    runtimeApi: RuntimeIngress = ProcessRuntimeIngress,
) : GeneratedRuntimeBridge(
    sessionHandle = sessionHandle,
    runtimeApi = runtimeApi,
), BackgroundEvents, NotificationEvents {
    private val mainHandler: Handler by lazy(LazyThreadSafetyMode.NONE) {
        Handler(Looper.getMainLooper())
    }
    private var runtimeHost: RuntimeHost? = null

    /**
     * Create one runtime bridge for one runtime session using the process ABI.
     */
    public constructor(
        sessionHandle: HostSessionHandle,
    ) : this(sessionHandle = sessionHandle, runtimeApi = ProcessRuntimeIngress)

    /**
     * Attach one runtime host and register the bridge callback lanes.
     */
    public fun attach(
        runtimeHost: RuntimeHost,
    ) {
        require(runtimeHost.sessionHandle == sessionHandle) {
            "runtime bridge session handle does not match the attached runtime host"
        }

        this.runtimeHost = runtimeHost

        val attachStatus = runtimeApi.attachBridge(
            sessionHandle = sessionHandle,
            bridge = this,
        )
        if (attachStatus != hostStatusOk) {
            this.runtimeHost = null
            throw IllegalStateException(
                "runtime bridge could not attach runtime session api: $attachStatus",
            )
        }
    }

    /**
     * Remove the bridge registration for this runtime session.
     */
    public fun detach() {
        runtimeApi.detachBridge(sessionHandle)
        runtimeHost = null
    }

    /**
     * Resolve the live runtime host for one bridge callback.
     */
    override fun <T> withRuntimeHost(
        onMissing: () -> T,
        body: (RuntimeHost) -> T,
    ): T {
        val runtimeHost = runtimeHost
            ?: return onMissing()

        return body(runtimeHost)
    }

    /**
     * Resolve the live runtime host on the Android main thread for one bridge callback.
     */
    override fun <T> withRuntimeHostOnMainThread(
        onMissing: () -> T,
        body: (RuntimeHost) -> T,
    ): T {
        return withRuntimeHost(
            onMissing = onMissing,
        ) { runtimeHost ->
            runOnMainThread { body(runtimeHost) }
        }
    }

    /**
     * Run one bridge callback on the Android main thread.
     */
    private fun <T> runOnMainThread(
        body: () -> T,
    ): T {
        // fast path
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return body()
        }

        // cross-thread handoff
        val completion = CountDownLatch(1)
        var value: T? = null
        var failure: Throwable? = null

        mainHandler.post {
            try {
                value = body()
            }
            catch (throwable: Throwable) {
                failure = throwable
            }
            finally {
                completion.countDown()
            }
        }

        // wait for the main-thread result
        try {
            completion.await()
        }
        catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            throw IllegalStateException(
                "runtime bridge main thread hop was interrupted",
                error,
            )
        }

        // surface the main-thread outcome
        failure?.let { throw it }

        @Suppress("UNCHECKED_CAST")
        return value as T
    }

}
