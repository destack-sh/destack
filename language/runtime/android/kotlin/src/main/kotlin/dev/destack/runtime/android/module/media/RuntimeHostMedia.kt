package dev.destack.runtime.android.module.media

/**
 * The default media page size when one request omits an explicit limit.
 */
internal const val runtimeHostMediaDefaultPageLimit: Int = 50

/**
 * The Android media asset kind returned by one host surface.
 */
public enum class RuntimeHostMediaAssetKind {
    /**
     * One image asset.
     */
    Image,

    /**
     * One video asset.
     */
    Video,

    /**
     * One audio asset.
     */
    Audio,

    /**
     * One non-standard asset.
     */
    Other,
}

/**
 * One media-asset descriptor returned by the Android host.
 */
public data class RuntimeHostMediaAssetDescriptor(
    /**
     * The stable host media identifier.
     */
    val identifier: String,

    /**
     * The host URI for this asset.
     */
    val uri: String,

    /**
     * The asset filename payload.
     */
    val filename: String,

    /**
     * The asset MIME type payload.
     */
    val mimeType: String,

    /**
     * The asset class.
     */
    val kind: RuntimeHostMediaAssetKind,

    /**
     * The asset width in pixels when available.
     */
    val width: Int = 0,

    /**
     * The asset height in pixels when available.
     */
    val height: Int = 0,

    /**
     * The asset duration in milliseconds for time-based assets.
     */
    val durationMs: Long = 0,

    /**
     * The asset size in bytes when available.
     */
    val sizeBytes: Long = 0,

    /**
     * The asset creation timestamp in UTC nanoseconds when available.
     */
    val createdUnixNs: Long = 0,

    /**
     * The asset modification timestamp in UTC nanoseconds when available.
     */
    val modifiedUnixNs: Long = 0,
)

/**
 * One Android media-list request submitted by one runtime session.
 */
public data class RuntimeHostMediaListRequest(
    /**
     * The opaque next-page cursor from one prior media-list call.
     */
    val cursor: String? = null,

    /**
     * The maximum number of returned assets when available.
     */
    val limit: Int? = null,

    /**
     * The requested asset kinds, empty means all kinds.
     */
    val kinds: List<RuntimeHostMediaAssetKind> = emptyList(),

    /**
     * Whether hidden assets should be included.
     */
    val includeHidden: Boolean = false,
)

/**
 * One Android media page returned to one runtime session.
 */
public data class RuntimeHostMediaListResult(
    /**
     * The listed media assets.
     */
    val assets: List<RuntimeHostMediaAssetDescriptor>,

    /**
     * The opaque next-page cursor when available.
     */
    val nextCursor: String = "",

    /**
     * Whether more assets are available.
     */
    val hasMore: Boolean = false,
)

/**
 * One Android media-list response returned by the host.
 */
public data class RuntimeHostMediaListResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned media page when available.
     */
    val page: RuntimeHostMediaListResult? = null,
)

/**
 * One Android media-read response returned by the host.
 */
public data class RuntimeHostMediaReadResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned asset descriptor when available.
     */
    val descriptor: RuntimeHostMediaAssetDescriptor? = null,
)

/**
 * One Android media import request submitted by one runtime session.
 */
public data class RuntimeHostMediaImportPathRequest(
    /**
     * The local path to import into the host media library.
     */
    val path: String,

    /**
     * The requested asset kind.
     */
    val kind: RuntimeHostMediaAssetKind,
)

/**
 * One Android media import response returned by the host.
 */
public data class RuntimeHostMediaImportPathResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The imported asset identifier when available.
     */
    val identifier: String? = null,
)

/**
 * One Android media delete request submitted by one runtime session.
 */
public data class RuntimeHostMediaDeleteRequest(
    /**
     * The stable host media identifiers to delete.
     */
    val identifiers: List<String>,
)

/**
 * One Android media delete response returned by the host.
 */
public data class RuntimeHostMediaDeleteResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The number of deleted assets.
     */
    val deletedCount: Int = 0,
)
