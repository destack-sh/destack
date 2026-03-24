package dev.destack.runtime.android.embedder

import dev.destack.runtime.android.core.HostEmbedderId

import androidx.activity.result.ActivityResultCallback
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.contract.ActivityResultContract
import androidx.lifecycle.LifecycleOwner

private const val REGISTRATION_KEY_PREFIX: String = "destack.runtime"

/**
 * The stable launcher key for one Android activity-result flow.
 */
internal enum class ActivityResultLauncherKey {
    /**
     * The launcher for one permission request.
     */
    PermissionRequest,

    /**
     * The launcher for one single-document request.
     */
    DocumentPickSingle,

    /**
     * The launcher for one multi-document request.
     */
    DocumentPickMultiple,
}

/**
 * The activity-result owner for one attached Android runtime session.
 */
internal class ActivityResults(
    /**
     * The stable embedder identifier for this runtime embedder.
     */
    private val embedderId: HostEmbedderId,

    /**
     * The registry used to install Android activity-result launchers.
     */
    private val registry: ActivityResultRegistry,

    /**
     * The lifecycle owner used for launcher registration.
     */
    private val lifecycleOwner: LifecycleOwner,
) {
    private val launchers: MutableList<ActivityResultLauncher<*>> = mutableListOf()

    /**
     * Register one launcher inside this runtime embedder.
     */
    internal fun <I, O> registerLauncher(
        launcherKey: ActivityResultLauncherKey,
        contract: ActivityResultContract<I, O>,
        callback: ActivityResultCallback<O>,
    ): ActivityResultLauncher<I> {
        val key = buildLauncherKey(launcherKey)

        val launcher = registry.register(key, lifecycleOwner, contract, callback)

        launchers += launcher

        return launcher
    }

    /**
     * Unregister every launcher owned by this runtime embedder.
     */
    internal fun unregisterAll() {
        for (launcher in launchers) {
            launcher.unregister()
        }

        launchers.clear()
    }

    /**
     * Build one stable launcher key for this runtime session.
     */
    private fun buildLauncherKey(
        launcherKey: ActivityResultLauncherKey,
    ): String {
        val keySuffix = when (launcherKey) {
            ActivityResultLauncherKey.PermissionRequest -> "permission.request"
            ActivityResultLauncherKey.DocumentPickSingle -> "document.pick.single"
            ActivityResultLauncherKey.DocumentPickMultiple -> "document.pick.multiple"
        }

        return "$REGISTRATION_KEY_PREFIX.${embedderId.rawValue}.launcher.$keySuffix"
    }
}
