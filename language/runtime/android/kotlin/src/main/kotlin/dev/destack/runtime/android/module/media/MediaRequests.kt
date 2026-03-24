package dev.destack.runtime.android.module.media

import dev.destack.runtime.android.core.hostStatusNotSupported

/**
 * The media request surface attached to one Android runtime host.
 */
public interface MediaRequests {
    /**
     * List one page of media assets for one query.
     */
    public fun listMedia(
        request: RuntimeHostMediaListRequest,
    ): RuntimeHostMediaListResponse

    /**
     * Read one media asset descriptor by stable identifier.
     */
    public fun readMedia(
        identifier: String,
    ): RuntimeHostMediaReadResponse

    /**
     * Import one local path into the media library.
     */
    public fun importMediaPath(
        request: RuntimeHostMediaImportPathRequest,
    ): RuntimeHostMediaImportPathResponse

    /**
     * Delete one batch of media assets and return the deleted count.
     */
    public fun deleteMedia(
        request: RuntimeHostMediaDeleteRequest,
    ): RuntimeHostMediaDeleteResponse
}

/**
 * The explicit unsupported media request surface for one Android runtime host.
 */
public object UnsupportedMediaRequests : MediaRequests {
    override fun listMedia(
        request: RuntimeHostMediaListRequest,
    ): RuntimeHostMediaListResponse {
        return RuntimeHostMediaListResponse(status = hostStatusNotSupported)
    }

    override fun readMedia(
        identifier: String,
    ): RuntimeHostMediaReadResponse {
        return RuntimeHostMediaReadResponse(status = hostStatusNotSupported)
    }

    override fun importMediaPath(
        request: RuntimeHostMediaImportPathRequest,
    ): RuntimeHostMediaImportPathResponse {
        return RuntimeHostMediaImportPathResponse(status = hostStatusNotSupported)
    }

    override fun deleteMedia(
        request: RuntimeHostMediaDeleteRequest,
    ): RuntimeHostMediaDeleteResponse {
        return RuntimeHostMediaDeleteResponse(status = hostStatusNotSupported)
    }
}
