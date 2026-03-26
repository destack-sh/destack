package dev.destack.runtime.android

import androidx.lifecycle.Lifecycle

import dev.destack.runtime.android.embedder.ActivityEmbedder
import dev.destack.runtime.android.embedder.ApplicationEmbedder
import dev.destack.runtime.android.module.background.UnsupportedBackgroundRequests

import org.junit.Assert.assertSame
import org.junit.Test

/**
 * Attach one runtime host to one application-level Android context.
 */
class ApplicationEmbedderTest {
    /**
     * Bind the application-owned background surface into the runtime host.
     */
    @Test
    fun testApplicationEmbedderBindsBackgroundRequestsIntoRuntimeHost() {
        val runtimeHost = createRuntimeHost()
        val embedder = ApplicationEmbedder.attach(
            context = null,
            runtimeHost = runtimeHost,
        )

        // bind the application-owned surface into the runtime host
        assertSame(embedder.backgroundRequests, runtimeHost.backgroundRequests)
        assertSame(UnsupportedBackgroundRequests, runtimeHost.backgroundRequests)

        embedder.detach()
    }

    /**
     * Leave the application-owned background surface attached across activity teardown.
     */
    @Test
    fun testActivityEmbedderLeavesApplicationBackgroundRequestsAttached() {
        val runtimeHost = createRuntimeHost()
        val applicationEmbedder = ApplicationEmbedder.attach(
            context = null,
            runtimeHost = runtimeHost,
        )
        val owner = TestLifecycleOwner()
        val registry = RecordingActivityResultRegistry()
        val activityEmbedder = ActivityEmbedder.attach(
            lifecycleOwner = owner,
            activityResultRegistry = registry,
            runtimeHost = runtimeHost,
        )

        // drive one minimal lifecycle before teardown
        owner.handleEvent(Lifecycle.Event.ON_CREATE)
        owner.handleEvent(Lifecycle.Event.ON_START)

        activityEmbedder.detach()

        // keep the application-owned surface after activity teardown
        assertSame(applicationEmbedder.backgroundRequests, runtimeHost.backgroundRequests)

        applicationEmbedder.detach()
    }
}
