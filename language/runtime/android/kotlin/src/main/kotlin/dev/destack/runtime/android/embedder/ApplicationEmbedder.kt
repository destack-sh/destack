package dev.destack.runtime.android.embedder

import android.content.Context

import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.background.BackgroundRequests
import dev.destack.runtime.android.module.background.ProcessBackgroundHost

/**
 * The application-level embedder for one Android runtime host.
 */
public class ApplicationEmbedder private constructor(
    /**
     * The Android runtime host attached to this application.
     */
    public val runtimeHost: RuntimeHost,

    /**
     * The background request surface attached to this application.
     */
    public val backgroundRequests: BackgroundRequests,
) {
    /**
     * Detach the process-level background sink from this runtime host.
     */
    public fun detach() {
        // detach the process-level background sink
        (backgroundRequests as? ProcessBackgroundHost)?.detach()
    }

    public companion object {
        /**
         * Attach one runtime host to one application-level Android context.
         */
        public fun attach(
            context: Context?,
            runtimeHost: RuntimeHost,
        ): ApplicationEmbedder {
            // attach the process-level background host to the application context
            val backgroundRequests = ProcessBackgroundHost.attach(
                context = context?.applicationContext,
                events = runtimeHost.backgroundEvents,
            )

            // bind the application-backed background surface into the runtime host
            runtimeHost.backgroundRequests = backgroundRequests

            return ApplicationEmbedder(
                runtimeHost = runtimeHost,
                backgroundRequests = backgroundRequests,
            )
        }
    }
}
