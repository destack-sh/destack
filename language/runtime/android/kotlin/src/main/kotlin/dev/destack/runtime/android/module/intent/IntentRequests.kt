package dev.destack.runtime.android.module.intent

/**
 * The intent request surface attached to one Android runtime host.
 */
public interface IntentRequests {
    /**
     * Return whether one outbound URL can be opened by the Android host.
     */
    public fun canOpenUrl(
        url: String,
    ): Boolean

    /**
     * Open one outbound URL through the Android host.
     */
    public fun openUrl(
        url: String,
    ): Int

    /**
     * Open one outbound file path through the Android host.
     */
    public fun openPath(
        path: String,
    ): Int

    /**
     * Share one outbound text payload through the Android host.
     */
    public fun shareText(
        text: String,
        contentType: String? = null,
    ): Int

    /**
     * Share one outbound file path list through the Android host.
     */
    public fun sharePaths(
        paths: List<String>,
        contentType: String? = null,
    ): Int
}
