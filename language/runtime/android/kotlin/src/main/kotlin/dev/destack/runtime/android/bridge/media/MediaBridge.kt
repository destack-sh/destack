package dev.destack.runtime.android.bridge.media

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.media.RuntimeHostMediaAssetKind
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaListRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaReadResponse

/**
 * One media bridge lane for one attached Android runtime host.
 */
internal class MediaBridge(
    private val sessionHandle: HostSessionHandle,
) {
    /**
     * List one page of media through the attached runtime host.
     */
    fun listMedia(
        runtimeHost: RuntimeHost,
        cursor: String?,
        hasLimit: Boolean,
        limit: Int,
        kinds: IntArray,
        includeHidden: Boolean,
    ): RuntimeHostMediaListResponse {
        val decodedKinds = decodeMediaKinds(kinds)
            ?: return RuntimeHostMediaListResponse(status = hostStatusInvalidArgument)

        return runtimeHost.mediaRequests.listMedia(
            RuntimeHostMediaListRequest(
                cursor = cursor,
                limit = if (hasLimit) limit else null,
                kinds = decodedKinds,
                includeHidden = includeHidden,
            ),
        )
    }

    /**
     * Read one media asset descriptor through the attached runtime host.
     */
    fun readMedia(
        runtimeHost: RuntimeHost,
        identifier: String,
    ): RuntimeHostMediaReadResponse {
        return runtimeHost.mediaRequests.readMedia(identifier)
    }

    /**
     * Import one local path into the attached runtime host media library.
     */
    fun importMediaPath(
        runtimeHost: RuntimeHost,
        path: String,
        kind: Int,
    ): RuntimeHostMediaImportPathResponse {
        val mediaKind = decodeMediaKind(kind)
            ?: return RuntimeHostMediaImportPathResponse(status = hostStatusInvalidArgument)

        return runtimeHost.mediaRequests.importMediaPath(
            RuntimeHostMediaImportPathRequest(
                path = path,
                kind = mediaKind,
            ),
        )
    }

    /**
     * Delete one batch of media assets through the attached runtime host.
     */
    fun deleteMedia(
        runtimeHost: RuntimeHost,
        identifiers: Array<String>,
    ): RuntimeHostMediaDeleteResponse {
        return runtimeHost.mediaRequests.deleteMedia(
            RuntimeHostMediaDeleteRequest(
                identifiers = identifiers.asList(),
            ),
        )
    }
}

/**
 * Decode one raw bridge media kind into one runtime host media kind.
 */
private fun decodeMediaKind(
    rawValue: Int,
): RuntimeHostMediaAssetKind? {
    return when (rawValue) {
        1 -> RuntimeHostMediaAssetKind.Image
        2 -> RuntimeHostMediaAssetKind.Video
        3 -> RuntimeHostMediaAssetKind.Audio
        4 -> RuntimeHostMediaAssetKind.Other
        else -> null
    }
}

/**
 * Decode one raw bridge media-kind array into one runtime host media-kind list.
 */
private fun decodeMediaKinds(
    rawValues: IntArray,
): List<RuntimeHostMediaAssetKind>? {
    val kinds = mutableListOf<RuntimeHostMediaAssetKind>()

    for (rawValue in rawValues) {
        val kind = decodeMediaKind(rawValue) ?: return null
        kinds += kind
    }

    return kinds
}
