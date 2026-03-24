package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.bridge.document.DocumentAbi
import dev.destack.runtime.android.bridge.document.ProcessDocumentAbi
import dev.destack.runtime.android.bridge.intent.IntentAbi
import dev.destack.runtime.android.bridge.intent.ProcessIntentAbi
import dev.destack.runtime.android.bridge.location.LocationAbi
import dev.destack.runtime.android.bridge.location.ProcessLocationAbi
import dev.destack.runtime.android.bridge.notification.NotificationAbi
import dev.destack.runtime.android.bridge.notification.ProcessNotificationAbi
import dev.destack.runtime.android.bridge.permission.PermissionAbi
import dev.destack.runtime.android.bridge.permission.ProcessPermissionAbi
import dev.destack.runtime.android.core.HostSessionHandle

/**
 * The native runtime-host library loader for the Android bridge.
 */
internal object RuntimeHostLibraryLoader {
    init {
        val explicitLibraryPath = System.getenv("DESTACK_RUNTIME_HOST_ANDROID_LIBRARY")
        if (!explicitLibraryPath.isNullOrBlank()) {
            System.load(explicitLibraryPath)
        } else {
            System.loadLibrary("destack_runtime_host_android")
        }
    }

    /**
     * Load the Android runtime-host library into this process.
     */
    fun ensureLoaded() {}
}

/**
 * The default runtime ABI surface resolved through the Android JNI bridge.
 */
public object ProcessRuntimeAbi :
    RuntimeAbi,
    DocumentAbi by ProcessDocumentAbi,
    PermissionAbi by ProcessPermissionAbi,
    IntentAbi by ProcessIntentAbi,
    LocationAbi by ProcessLocationAbi,
    NotificationAbi by ProcessNotificationAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun attachBridge(
        sessionHandle: HostSessionHandle,
        bridge: RuntimeBridge,
    ): Int {
        return nativeAttachBridge(
            sessionHandle.rawValue,
            bridge,
        )
    }

    override fun detachBridge(
        sessionHandle: HostSessionHandle,
    ) {
        nativeDetachBridge(sessionHandle.rawValue)
    }

    /**
     * Attach one runtime bridge to one native runtime session.
     */
    private external fun nativeAttachBridge(
        sessionHandle: Long,
        bridge: RuntimeBridge,
    ): Int

    /**
     * Detach one runtime bridge from one native runtime session.
     */
    private external fun nativeDetachBridge(
        sessionHandle: Long,
    )
}
