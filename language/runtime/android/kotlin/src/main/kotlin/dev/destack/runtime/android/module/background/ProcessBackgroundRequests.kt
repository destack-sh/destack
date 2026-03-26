package dev.destack.runtime.android.module.background

import android.content.Context

/**
 * The compatibility wrapper around the process-level background host.
 */
public class ProcessBackgroundRequests private constructor(
    private val host: ProcessBackgroundHost,
) : BackgroundRequests by host {
    /**
     * Detach the live event sink from this request surface.
     */
    public fun detach() {
        host.detach()
    }

    public companion object {
        /**
         * Attach one process-level background surface for the given context and event sink.
         */
        public fun attach(
            context: Context?,
            events: BackgroundEvents,
        ): BackgroundRequests {
            val requests = ProcessBackgroundHost.attach(context, events)

            return if (requests is ProcessBackgroundHost) {
                ProcessBackgroundRequests(requests)
            }
            else {
                requests
            }
        }
    }
}
