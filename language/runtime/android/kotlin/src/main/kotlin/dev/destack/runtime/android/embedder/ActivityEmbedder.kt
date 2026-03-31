package dev.destack.runtime.android.embedder

import android.os.Bundle

import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultRegistry
import androidx.lifecycle.LifecycleOwner
import androidx.savedstate.SavedStateRegistryOwner

import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.document.DocumentActivityResults
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.location.ActivityLocationRequests
import dev.destack.runtime.android.module.location.LocationRequests
import dev.destack.runtime.android.module.permission.PermissionActivityResults
import dev.destack.runtime.android.module.permission.PermissionRequests
import dev.destack.runtime.android.module.text.ActivityTextRequests
import dev.destack.runtime.android.module.text.TextRequests

private const val documentRequestIdKey: String = "documentRequestId"
private const val permissionRequestIdKey: String = "permissionRequestId"
private const val permissionNameKey: String = "permissionName"

/**
 * The framework embedder between one Android activity environment and one runtime host.
 */
public class ActivityEmbedder private constructor(
    /**
     * The lifecycle owner for this activity embedder.
     */
    private val lifecycleOwner: LifecycleOwner,

    /**
     * The Android runtime host attached to this activity.
     */
    public val runtimeHost: RuntimeHost,

    /**
     * The activity lifecycle observer installed for this embedder.
     */
    private val lifecycleObserver: ActivityObserver,

    /**
     * The activity-result launcher owner for this embedder.
     */
    private val activityResults: ActivityResults,

    /**
     * The permission surface attached to this activity.
     */
    public val permission: PermissionRequests,

    /**
     * The document surface attached to this activity.
     */
    public val document: DocumentRequests,

    /**
     * The location surface attached to this activity.
     */
    public val location: LocationRequests,

    /**
     * The text surface attached to this activity.
     */
    public val text: TextRequests,

    /**
     * The optional state store that owns interactive-flow persistence.
     */
    private val savedInteractiveState: SavedInteractiveState? = null,
) {
    /**
     * Remove the lifecycle observer from the attached activity.
     */
    public fun detach() {
        // detach interactive-flow persistence
        savedInteractiveState?.detach()

        // unregister activity-result launchers
        activityResults.unregisterAll()

        // detach the hidden text editor
        (text as? ActivityTextRequests)?.detach()

        // remove lifecycle ingress
        lifecycleOwner.lifecycle.removeObserver(lifecycleObserver)
    }

    public companion object {
        /**
         * Attach one runtime host to one `ComponentActivity`.
         */
        public fun attach(
            activity: ComponentActivity,
            runtimeHost: RuntimeHost,
        ): ActivityEmbedder {
            return attach(
                lifecycleOwner = activity,
                savedInteractiveState = SavedInteractiveState(
                    embedderId = runtimeHost.embedderId,
                    owner = activity,
                ),
                activityResultRegistry = activity.activityResultRegistry,
                runtimeHost = runtimeHost,
            )
        }

        /**
         * Attach one runtime host to one activity-equivalent environment.
         */
        public fun attach(
            lifecycleOwner: LifecycleOwner,
            savedStateOwner: SavedStateRegistryOwner? = lifecycleOwner as? SavedStateRegistryOwner,
            activityResultRegistry: ActivityResultRegistry,
            runtimeHost: RuntimeHost,
        ): ActivityEmbedder {
            val savedInteractiveState = savedStateOwner?.let { owner ->
                SavedInteractiveState(
                    embedderId = runtimeHost.embedderId,
                    owner = owner,
                )
            }

            return attach(
                lifecycleOwner = lifecycleOwner,
                savedInteractiveState = savedInteractiveState,
                activityResultRegistry = activityResultRegistry,
                runtimeHost = runtimeHost,
            )
        }

        /**
         * Attach one runtime host using one concrete interactive-state helper.
         */
        internal fun attach(
            lifecycleOwner: LifecycleOwner,
            savedInteractiveState: SavedInteractiveState?,
            activityResultRegistry: ActivityResultRegistry,
            runtimeHost: RuntimeHost,
        ): ActivityEmbedder {

            // restore in-flight interactive flows before launcher registration
            savedInteractiveState?.restore(runtimeHost)

            val activityResults = ActivityResults(
                embedderId = runtimeHost.embedderId,
                registry = activityResultRegistry,
                lifecycleOwner = lifecycleOwner,
            )
            val permission = PermissionActivityResults(
                runtimeHost = runtimeHost,
                activity = lifecycleOwner as? ComponentActivity,
                activityResults = activityResults,
                permissionEvents = runtimeHost.permission,
            )
            val document = DocumentActivityResults(
                runtimeHost = runtimeHost,
                activityResults = activityResults,
            )
            val location = ActivityLocationRequests(
                activity = lifecycleOwner as? ComponentActivity,
                events = runtimeHost.location,
            )
            val text = ActivityTextRequests(
                activity = lifecycleOwner as? ComponentActivity,
                events = runtimeHost.text,
            )
            val lifecycleObserver = ActivityObserver(runtimeHost)

            // bind the activity-backed text surface into the runtime host
            runtimeHost.updateTextRequests(text)

            // persist in-flight interactive flows for recreation
            savedInteractiveState?.attach(runtimeHost)

            // install lifecycle ingress
            lifecycleOwner.lifecycle.addObserver(lifecycleObserver)

            return ActivityEmbedder(
                lifecycleOwner = lifecycleOwner,
                runtimeHost = runtimeHost,
                lifecycleObserver = lifecycleObserver,
                activityResults = activityResults,
                permission = permission,
                document = document,
                location = location,
                text = text,
                savedInteractiveState = savedInteractiveState,
            )
        }
    }
}

/**
 * The Android saved-state adapter for one embedder interactive flow set.
 */
internal class SavedInteractiveState(
    /**
     * The stable embedder identifier for this runtime embedder.
     */
    val embedderId: HostEmbedderId? = null,

    /**
     * The saved-state owner for this embedder.
     */
    val owner: SavedStateRegistryOwner? = null,
) {
    private data class PendingPermissionRequest(
        val requestId: HostRequestId,
        val permission: String,
    )

    private data class RecordedInteractiveState(
        val documentRequestId: HostRequestId? = null,
        val permissionRequest: PendingPermissionRequest? = null,
    )

    private val stateKey: String? = embedderId?.let { embedderId ->
        "dev.destack.runtime.${embedderId.rawValue}.interactive"
    }
    private var runtimeHost: RuntimeHost? = null
    private var recordedState: RecordedInteractiveState? = null

    /**
     * Restore the in-flight interactive requests owned by this embedder.
     */
    fun restore(
        runtimeHost: RuntimeHost,
    ) {
        val recordedState = if (owner == null) {
            recordedState
        } else {
            decodeRecordedState(consumePendingRequestBundle())
        }

        runtimeHost.restoreDocumentRequest(recordedState?.documentRequestId)
        runtimeHost.restorePermissionRequest(
            recordedState?.permissionRequest?.requestId,
            recordedState?.permissionRequest?.permission,
        )

        this.recordedState = null
    }

    /**
     * Attach one live runtime host to this saved-state adapter.
     */
    fun attach(
        runtimeHost: RuntimeHost,
    ) {
        this.runtimeHost = runtimeHost

        val owner = owner ?: return
        val stateKey = requireNotNull(stateKey)

        owner.savedStateRegistry.registerSavedStateProvider(stateKey) {
            pendingRequestBundle(runtimeHost)
        }
    }

    /**
     * Detach the saved-state provider for this embedder.
     */
    fun detach() {
        recordedState = runtimeHost?.let(::recordedState)
        runtimeHost = null

        if (owner != null) {
            owner.savedStateRegistry.unregisterSavedStateProvider(requireNotNull(stateKey))
        }
    }

    /**
     * Encode the in-flight interactive request state into one bundle.
     */
    private fun pendingRequestBundle(
        runtimeHost: RuntimeHost,
    ): Bundle {
        return pendingRequestBundle(recordedState(runtimeHost))
    }

    /**
     * Encode one recorded interactive state into one bundle.
     */
    private fun pendingRequestBundle(
        recordedState: RecordedInteractiveState?,
    ): Bundle {
        val documentRequestId = recordedState?.documentRequestId
        val permissionRequest = recordedState?.permissionRequest

        return Bundle().apply {
            // document
            if (documentRequestId != null) {
                putLong(documentRequestIdKey, documentRequestId.rawValue)
            }

            // permission
            if (permissionRequest != null) {
                putLong(permissionRequestIdKey, permissionRequest.requestId.rawValue)
                putString(permissionNameKey, permissionRequest.permission)
            }
        }
    }

    /**
     * Consume the current pending-request bundle.
     */
    private fun consumePendingRequestBundle(): Bundle {
        if (owner != null) {
            val stateKey = requireNotNull(stateKey)

            return owner.savedStateRegistry.consumeRestoredStateForKey(stateKey) ?: Bundle()
        }

        return pendingRequestBundle(recordedState)
    }

    /**
     * Snapshot the live interactive state from one runtime host.
     */
    private fun recordedState(
        runtimeHost: RuntimeHost,
    ): RecordedInteractiveState? {
        val documentRequestId = runtimeHost.pendingDocumentRequestId()
        val permissionRequestId = runtimeHost.pendingPermissionRequestId()
        val permissionName = runtimeHost.pendingPermission()

        val permissionRequest = if (permissionRequestId == null || permissionName == null) {
            null
        } else {
            PendingPermissionRequest(
                requestId = permissionRequestId,
                permission = permissionName,
            )
        }

        if (documentRequestId == null && permissionRequest == null) {
            return null
        }

        return RecordedInteractiveState(
            documentRequestId = documentRequestId,
            permissionRequest = permissionRequest,
        )
    }

    /**
     * Decode one recorded interactive state from one saved bundle.
     */
    private fun decodeRecordedState(
        bundle: Bundle,
    ): RecordedInteractiveState? {
        val documentRequestId = if (bundle.containsKey(documentRequestIdKey)) {
            HostRequestId(rawValue = bundle.getLong(documentRequestIdKey))
        } else {
            null
        }

        val hasRequestId = bundle.containsKey(permissionRequestIdKey)
        val hasPermissionName = bundle.containsKey(permissionNameKey)

        // reject partial saved permission state
        require(hasRequestId == hasPermissionName) {
            "interactive permission saved state is corrupted for Android runtime embedder"
        }

        val permissionRequest = if (!hasRequestId) {
            null
        } else {
            PendingPermissionRequest(
                requestId = HostRequestId(rawValue = bundle.getLong(permissionRequestIdKey)),
                permission = requireNotNull(bundle.getString(permissionNameKey)) {
                    "interactive permission saved state is corrupted for Android runtime embedder"
                },
            )
        }

        if (documentRequestId == null && permissionRequest == null) {
            return null
        }

        return RecordedInteractiveState(
            documentRequestId = documentRequestId,
            permissionRequest = permissionRequest,
        )
    }
}
