package dev.destack.runtime.android.bridge.core

import android.os.Handler
import android.os.Looper

import java.util.concurrent.CountDownLatch

/**
 * One helper that runs bridge work on the Android main thread.
 */
internal class MainThreadBridge {
    private val mainHandler: Handler by lazy(LazyThreadSafetyMode.NONE) {
        Handler(Looper.getMainLooper())
    }

    /**
     * Run one bridge operation on the main thread and return its result.
     */
    fun <T> run(
        body: () -> T,
    ): T {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return body()
        }

        val latch = CountDownLatch(1)
        var result: T? = null
        var error: Throwable? = null

        mainHandler.post {
            try {
                result = body()
            }
            catch (throwable: Throwable) {
                error = throwable
            }
            finally {
                latch.countDown()
            }
        }

        try {
            latch.await()
        }
        catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            throw IllegalStateException(
                "runtime bridge main thread hop was interrupted",
                error,
            )
        }

        error?.let { throw it }

        @Suppress("UNCHECKED_CAST")
        return result as T
    }
}
