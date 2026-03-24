package dev.destack.runtime.android.module.intent

/**
 * The Android host intent payload delivered into one runtime session.
 */
public sealed interface RuntimeHostIntentPayload {
    /**
     * One request to open one URL.
     */
    public data class OpenUrl(
        /**
         * The URL delivered by the Android host.
         */
        val url: String,
    ) : RuntimeHostIntentPayload

    /**
     * One request to open one file path.
     */
    public data class OpenFile(
        /**
         * The path delivered by the Android host.
         */
        val path: String,

        /**
         * The normalized content type when available.
         */
        val contentType: String? = null,
    ) : RuntimeHostIntentPayload

    /**
     * One request to deliver shared text.
     */
    public data class ShareText(
        /**
         * The shared text payload.
         */
        val text: String,

        /**
         * The normalized content type when available.
         */
        val contentType: String? = null,
    ) : RuntimeHostIntentPayload

    /**
     * One request to deliver shared file paths.
     */
    public data class ShareFiles(
        /**
         * The shared file-path payloads.
         */
        val paths: List<String>,

        /**
         * The normalized content type when available.
         */
        val contentType: String? = null,
    ) : RuntimeHostIntentPayload

    /**
     * One custom action payload delivered by the Android host.
     */
    public data class CustomAction(
        /**
         * The custom action identifier.
         */
        val action: String,

        /**
         * The optional URL payload.
         */
        val url: String? = null,

        /**
         * The optional file-path payloads.
         */
        val paths: List<String> = emptyList(),

        /**
         * The optional shared text payload.
         */
        val text: String? = null,

        /**
         * The normalized content type when available.
         */
        val contentType: String? = null,
    ) : RuntimeHostIntentPayload
}

/**
 * One Android intent ingress event delivered into one runtime session.
 */
public data class RuntimeHostIntentEvent(
    /**
     * The source package or process identifier when available.
     */
    val source: String? = null,

    /**
     * The intent payload delivered by the Android host.
     */
    val payload: RuntimeHostIntentPayload,
)
