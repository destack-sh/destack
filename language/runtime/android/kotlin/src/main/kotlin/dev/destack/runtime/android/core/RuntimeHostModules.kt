package dev.destack.runtime.android.core

import dev.destack.runtime.android.module.background.BackgroundEvents
import dev.destack.runtime.android.module.background.BackgroundRequests
import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.text.TextEvents
import dev.destack.runtime.android.module.text.TextRequests

/**
 * The lifecycle host surface attached to one Android runtime host.
 */
public class LifecycleHost(
    events: LifecycleEvents,
) : LifecycleEvents by events

/**
 * The background host surface attached to one Android runtime host.
 */
public class BackgroundHost(
    requests: BackgroundRequests,
    events: BackgroundEvents,
) : BackgroundRequests, BackgroundEvents by events {
    private var requests: BackgroundRequests = requests

    /**
     * Replace the request surface for this host.
     */
    internal fun updateRequests(
        requests: BackgroundRequests,
    ) {
        this.requests = requests
    }

    /**
     * Read the Android background scheduler status.
     */
    override fun status() = requests.status()

    /**
     * List registered Android background tasks.
     */
    override fun list() = requests.list()

    /**
     * Register one Android background task.
     */
    override fun registerTask(request: dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskOptions) =
        requests.registerTask(request)

    /**
     * Unregister one Android background task.
     */
    override fun unregister(request: dev.destack.runtime.android.module.background.RuntimeHostBackgroundUnregisterRequest) =
        requests.unregister(request)

    /**
     * Trigger one Android background task for testing.
     */
    override fun triggerTest(request: dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerTestRequest) =
        requests.triggerTest(request)

    /**
     * Complete one Android background task execution.
     */
    override fun complete(request: dev.destack.runtime.android.module.background.RuntimeHostBackgroundCompleteRequest) =
        requests.complete(request)
}

/**
 * The text host surface attached to one Android runtime host.
 */
public class TextHost(
    requests: TextRequests,
    private val events: TextEvents,
) : TextRequests, TextEvents {
    private var requests: TextRequests = requests

    /**
     * Replace the request surface for this host.
     */
    internal fun updateRequests(
        requests: TextRequests,
    ) {
        this.requests = requests
    }

    /**
     * Deliver one text-session state event into one runtime session.
     */
    override fun notifyTextInputState(
        event: dev.destack.runtime.android.module.text.RuntimeHostTextInputEvent,
    ) {
        events.notifyTextInputState(event)
    }

    /**
     * Open one text session through one attached host.
     */
    override fun open(
        request: dev.destack.runtime.android.module.text.RuntimeHostTextInputOpenRequest,
    ) = requests.open(request)

    /**
     * Close one text session through one attached host.
     */
    override fun close(
        request: dev.destack.runtime.android.module.text.RuntimeHostTextInputCloseRequest,
    ) = requests.close(request)

    /**
     * Update one text geometry payload through one attached host.
     */
    override fun setGeometry(
        request: dev.destack.runtime.android.module.text.RuntimeHostTextInputGeometryRequest,
    ) = requests.setGeometry(request)

    /**
     * Update one text state payload through one attached host.
     */
    override fun setState(
        request: dev.destack.runtime.android.module.text.RuntimeHostTextInputStateRequest,
    ) = requests.setState(request)
}
