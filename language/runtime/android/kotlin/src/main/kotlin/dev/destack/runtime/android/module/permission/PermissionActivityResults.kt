package dev.destack.runtime.android.module.permission

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.provider.Settings

import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultCallback
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts

import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.embedder.ActivityResults
import dev.destack.runtime.android.embedder.ActivityResultLauncherKey
import dev.destack.runtime.android.core.RuntimeHost

/**
 * The permission activity-result bridge for one Android runtime host.
 */
internal class PermissionActivityResults(
    private val runtimeHost: RuntimeHost,
    private val activity: ComponentActivity?,
    activityResults: ActivityResults,
    private val permissionEvents: PermissionEvents,
) : PermissionRequests {
    private val permissionLauncher: ActivityResultLauncher<String> =
        activityResults.registerLauncher(
            launcherKey = ActivityResultLauncherKey.PermissionRequest,
            contract = ActivityResultContracts.RequestPermission(),
            callback = ActivityResultCallback(::completePermissionRequest),
        )

    /**
     * Submit one permission request through the Android activity-result registry.
     */
    override fun request(
        request: RuntimeHostPermissionRequest,
    ): Int {
        val permissionName = androidPermissionName(request.permission)
            ?: return hostStatusNotSupported

        runtimeHost.beginPermissionRequest(
            requestId = request.requestId,
            permission = request.permission,
        )
        permissionLauncher.launch(permissionName)

        return hostStatusOk
    }

    /**
     * Open the Android application settings surface for this runtime host.
     */
    override fun openSettings(): Int {
        val activity = activity
            ?: return hostStatusNotSupported

        val intent = Intent(
            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
            Uri.fromParts("package", activity.packageName, null),
        )

        return try {
            activity.startActivity(intent)
            hostStatusOk
        } catch (_: ActivityNotFoundException) {
            hostStatusNotSupported
        }
    }

    /**
     * Finish one in-flight permission request.
     */
    private fun completePermissionRequest(
        isGranted: Boolean,
    ) {
        val (requestId, permission) = runtimeHost.finishPermissionRequest()

        permissionEvents.notifyPermissionResult(
            RuntimeHostPermissionEvent(
                requestId = requestId,
                permission = permission,
                isGranted = isGranted,
            ),
        )
    }
}
