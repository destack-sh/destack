package dev.destack.runtime.android.module.media

import android.content.ContentResolver
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.provider.MediaStore
import android.webkit.MimeTypeMap
import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk

import java.io.File
import java.io.FileInputStream
import java.net.URLConnection

private const val pendingValueReady: Int = 0
private const val pendingValuePending: Int = 1
private const val relativePathImages: String = "Pictures/Destack"
private const val relativePathVideos: String = "Movies/Destack"
private const val relativePathAudio: String = "Music/Destack"
private const val relativePathOther: String = "Download/Destack"
private const val nanosecondsPerSecond: Long = 1_000_000_000L

/**
 * The Android media request surface backed by one attached context.
 */
public class ContextMediaRequests(
    context: Context,
) : MediaRequests {
    private val contentResolver: ContentResolver = context.contentResolver

    override fun list(
        request: RuntimeHostMediaListRequest,
    ): RuntimeHostMediaListResponse {
        return list(contentResolver, request)
    }

    override fun read(
        identifier: String,
    ): RuntimeHostMediaReadResponse {
        return read(contentResolver, identifier)
    }

    override fun importPath(
        request: RuntimeHostMediaImportPathRequest,
    ): RuntimeHostMediaImportPathResponse {
        return importPath(contentResolver, request)
    }

    override fun delete(
        request: RuntimeHostMediaDeleteRequest,
    ): RuntimeHostMediaDeleteResponse {
        return delete(contentResolver, request)
    }
}

private fun list(
    contentResolver: ContentResolver,
    request: RuntimeHostMediaListRequest,
): RuntimeHostMediaListResponse {
    // decode the cursor and page size
    val offset = request.cursor?.toIntOrNull()
    if (request.cursor != null && offset == null) {
        return RuntimeHostMediaListResponse(status = hostStatusInvalidArgument)
    }

    val pageLimit = request.limit ?: runtimeHostMediaDefaultPageLimit
    if (pageLimit <= 0) {
        return RuntimeHostMediaListResponse(status = hostStatusInvalidArgument)
    }

    // query one extra row to determine whether more assets exist
    val queryLimit = pageLimit + 1
    val projection = mediaProjection()
    val queryUri = MediaStore.Files.getContentUri(MediaStore.VOLUME_EXTERNAL)
    val selection = mediaSelection(request.kinds, request.includeHidden)
    val selectionArguments = mediaSelectionArguments(request.kinds)
    val sortOrder = mediaSortOrder(queryLimit, offset ?: 0)

    return try {
        contentResolver.query(
            queryUri,
            projection,
            selection,
            selectionArguments,
            sortOrder,
        )?.use { cursor ->
            val assets = mutableListOf<RuntimeHostMediaAssetDescriptor>()

            while (cursor.moveToNext()) {
                assets += decodeMediaAssetDescriptor(cursor)
            }

            val hasMore = assets.size > pageLimit
            val pageAssets = if (hasMore) {
                assets.dropLast(1)
            } else {
                assets
            }
            val nextCursor = if (hasMore) {
                ((offset ?: 0) + pageAssets.size).toString()
            } else {
                ""
            }

            RuntimeHostMediaListResponse(
                status = hostStatusOk,
                page = RuntimeHostMediaListResult(
                    assets = pageAssets,
                    nextCursor = nextCursor,
                    hasMore = hasMore,
                ),
            )
        } ?: RuntimeHostMediaListResponse(status = hostStatusFailed)
    }
    catch (_: Exception) {
        RuntimeHostMediaListResponse(status = hostStatusFailed)
    }
}

private fun read(
    contentResolver: ContentResolver,
    identifier: String,
): RuntimeHostMediaReadResponse {
    // resolve one stable media identifier into one row lookup
    val mediaId = identifier.toLongOrNull()
        ?: return RuntimeHostMediaReadResponse(status = hostStatusInvalidArgument)
    val itemUri = ContentUris.withAppendedId(
        MediaStore.Files.getContentUri(MediaStore.VOLUME_EXTERNAL),
        mediaId,
    )

    return try {
        contentResolver.query(
            itemUri,
            mediaProjection(),
            null,
            null,
            null,
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                return@use RuntimeHostMediaReadResponse(status = hostStatusNotFound)
            }

            RuntimeHostMediaReadResponse(
                status = hostStatusOk,
                descriptor = decodeMediaAssetDescriptor(cursor),
            )
        } ?: RuntimeHostMediaReadResponse(status = hostStatusFailed)
    }
    catch (_: Exception) {
        RuntimeHostMediaReadResponse(status = hostStatusFailed)
    }
}

private fun importPath(
    contentResolver: ContentResolver,
    request: RuntimeHostMediaImportPathRequest,
): RuntimeHostMediaImportPathResponse {
    // validate the source path before touching the media store
    val sourceFile = File(request.path)
    if (!sourceFile.exists() || !sourceFile.isFile) {
        return RuntimeHostMediaImportPathResponse(status = hostStatusInvalidArgument)
    }

    val mimeType = inferMediaMimeType(sourceFile, request.kind)
    val targetUri = mediaInsertUri(request.kind)
    val insertValues = ContentValues().apply {
        put(MediaStore.MediaColumns.DISPLAY_NAME, sourceFile.name)
        put(MediaStore.MediaColumns.MIME_TYPE, mimeType)
        put(MediaStore.MediaColumns.RELATIVE_PATH, mediaRelativePath(request.kind))
        put(MediaStore.MediaColumns.IS_PENDING, pendingValuePending)
    }

    return try {
        val insertedUri = contentResolver.insert(targetUri, insertValues)
            ?: return RuntimeHostMediaImportPathResponse(status = hostStatusFailed)

        FileInputStream(sourceFile).use { input ->
            contentResolver.openOutputStream(insertedUri)?.use { output ->
                input.copyTo(output)
            } ?: run {
                contentResolver.delete(insertedUri, null, null)

                return RuntimeHostMediaImportPathResponse(status = hostStatusFailed)
            }
        }

        val publishValues = ContentValues().apply {
            put(MediaStore.MediaColumns.IS_PENDING, pendingValueReady)
        }
        contentResolver.update(insertedUri, publishValues, null, null)

        RuntimeHostMediaImportPathResponse(
            status = hostStatusOk,
            identifier = ContentUris.parseId(insertedUri).toString(),
        )
    }
    catch (_: UnsupportedOperationException) {
        RuntimeHostMediaImportPathResponse(status = hostStatusNotSupported)
    }
    catch (_: Exception) {
        RuntimeHostMediaImportPathResponse(status = hostStatusFailed)
    }
}

private fun delete(
    contentResolver: ContentResolver,
    request: RuntimeHostMediaDeleteRequest,
): RuntimeHostMediaDeleteResponse {
    // validate the incoming identifier batch before mutating host state
    val mediaIds = request.identifiers.map { identifier ->
        identifier.toLongOrNull() ?: return RuntimeHostMediaDeleteResponse(
            status = hostStatusInvalidArgument,
        )
    }

    return try {
        var deletedCount = 0

        for (mediaId in mediaIds) {
            val itemUri = ContentUris.withAppendedId(
                MediaStore.Files.getContentUri(MediaStore.VOLUME_EXTERNAL),
                mediaId,
            )
            deletedCount += contentResolver.delete(itemUri, null, null)
        }

        RuntimeHostMediaDeleteResponse(
            status = hostStatusOk,
            deletedCount = deletedCount,
        )
    }
    catch (_: UnsupportedOperationException) {
        RuntimeHostMediaDeleteResponse(status = hostStatusNotSupported)
    }
    catch (_: SecurityException) {
        RuntimeHostMediaDeleteResponse(status = hostStatusNotSupported)
    }
    catch (_: Exception) {
        RuntimeHostMediaDeleteResponse(status = hostStatusFailed)
    }
}

private fun mediaProjection(): Array<String> {
    return arrayOf(
        MediaStore.MediaColumns._ID,
        MediaStore.MediaColumns.DISPLAY_NAME,
        MediaStore.MediaColumns.MIME_TYPE,
        MediaStore.Files.FileColumns.MEDIA_TYPE,
        MediaStore.MediaColumns.WIDTH,
        MediaStore.MediaColumns.HEIGHT,
        MediaStore.MediaColumns.DURATION,
        MediaStore.MediaColumns.SIZE,
        MediaStore.MediaColumns.DATE_ADDED,
        MediaStore.MediaColumns.DATE_MODIFIED,
    )
}

private fun mediaSelection(
    kinds: List<RuntimeHostMediaAssetKind>,
    includeHidden: Boolean,
): String? {
    val filters = mutableListOf<String>()

    // limit one query to the requested media kinds when a filter is present
    if (kinds.isNotEmpty()) {
        val placeholders = List(kinds.size) { "?" }.joinToString(", ")
        filters += "${MediaStore.Files.FileColumns.MEDIA_TYPE} IN ($placeholders)"
    }

    // hide pending assets unless the caller explicitly asks for hidden rows
    if (!includeHidden) {
        filters += "${MediaStore.MediaColumns.IS_PENDING} = $pendingValueReady"
    }

    if (filters.isEmpty()) {
        return null
    }

    return filters.joinToString(" AND ")
}

private fun mediaSelectionArguments(
    kinds: List<RuntimeHostMediaAssetKind>,
): Array<String>? {
    if (kinds.isEmpty()) {
        return null
    }

    return kinds
        .map { mediaStoreKind(it).toString() }
        .toTypedArray()
}

private fun mediaSortOrder(
    limit: Int,
    offset: Int,
): String {
    return buildString {
        append("${MediaStore.MediaColumns.DATE_MODIFIED} DESC")
        append(", ${MediaStore.MediaColumns._ID} DESC")
        append(" LIMIT ")
        append(limit)
        append(" OFFSET ")
        append(offset)
    }
}

private fun decodeMediaAssetDescriptor(
    cursor: android.database.Cursor,
): RuntimeHostMediaAssetDescriptor {
    val identifier = cursor.getLong(
        cursor.getColumnIndexOrThrow(MediaStore.MediaColumns._ID),
    )
    val displayName = cursor.getStringOrNull(MediaStore.MediaColumns.DISPLAY_NAME)
        ?: identifier.toString()
    val mimeType = cursor.getStringOrNull(MediaStore.MediaColumns.MIME_TYPE)
        ?: inferMediaMimeType(displayName)
    val mediaType = cursor.getIntOrDefault(MediaStore.Files.FileColumns.MEDIA_TYPE)
    val width = cursor.getIntOrDefault(MediaStore.MediaColumns.WIDTH)
    val height = cursor.getIntOrDefault(MediaStore.MediaColumns.HEIGHT)
    val durationMs = cursor.getLongOrDefault(MediaStore.MediaColumns.DURATION)
    val sizeBytes = cursor.getLongOrDefault(MediaStore.MediaColumns.SIZE)
    val createdUnixNs = cursor.getLongOrDefault(MediaStore.MediaColumns.DATE_ADDED) *
        nanosecondsPerSecond
    val modifiedUnixNs = cursor.getLongOrDefault(MediaStore.MediaColumns.DATE_MODIFIED) *
        nanosecondsPerSecond
    val uri = ContentUris.withAppendedId(
        MediaStore.Files.getContentUri(MediaStore.VOLUME_EXTERNAL),
        identifier,
    )

    return RuntimeHostMediaAssetDescriptor(
        identifier = identifier.toString(),
        uri = uri.toString(),
        filename = displayName,
        mimeType = mimeType,
        kind = runtimeMediaKind(mediaType),
        width = width,
        height = height,
        durationMs = durationMs,
        sizeBytes = sizeBytes,
        createdUnixNs = createdUnixNs,
        modifiedUnixNs = modifiedUnixNs,
    )
}

private fun inferMediaMimeType(
    sourceFile: File,
    kind: RuntimeHostMediaAssetKind,
): String {
    return inferMediaMimeType(sourceFile.name, kind)
}

private fun inferMediaMimeType(
    filename: String,
    kind: RuntimeHostMediaAssetKind? = null,
): String {
    val fromName = URLConnection.guessContentTypeFromName(filename)
    if (fromName != null) {
        return fromName
    }

    val extension = filename.substringAfterLast('.', missingDelimiterValue = "")
    if (extension.isNotBlank()) {
        val fromExtension = MimeTypeMap.getSingleton()
            .getMimeTypeFromExtension(extension.lowercase())
        if (fromExtension != null) {
            return fromExtension
        }
    }

    return when (kind) {
        RuntimeHostMediaAssetKind.Image -> "image/*"
        RuntimeHostMediaAssetKind.Video -> "video/*"
        RuntimeHostMediaAssetKind.Audio -> "audio/*"
        else -> "application/octet-stream"
    }
}

private fun mediaInsertUri(
    kind: RuntimeHostMediaAssetKind,
): Uri {
    return when (kind) {
        RuntimeHostMediaAssetKind.Image -> MediaStore.Images.Media.EXTERNAL_CONTENT_URI
        RuntimeHostMediaAssetKind.Video -> MediaStore.Video.Media.EXTERNAL_CONTENT_URI
        RuntimeHostMediaAssetKind.Audio -> MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
        RuntimeHostMediaAssetKind.Other -> MediaStore.Files.getContentUri(MediaStore.VOLUME_EXTERNAL)
    }
}

private fun mediaRelativePath(
    kind: RuntimeHostMediaAssetKind,
): String {
    return when (kind) {
        RuntimeHostMediaAssetKind.Image -> relativePathImages
        RuntimeHostMediaAssetKind.Video -> relativePathVideos
        RuntimeHostMediaAssetKind.Audio -> relativePathAudio
        RuntimeHostMediaAssetKind.Other -> relativePathOther
    }
}

private fun mediaStoreKind(
    kind: RuntimeHostMediaAssetKind,
): Int {
    return when (kind) {
        RuntimeHostMediaAssetKind.Image -> MediaStore.Files.FileColumns.MEDIA_TYPE_IMAGE
        RuntimeHostMediaAssetKind.Video -> MediaStore.Files.FileColumns.MEDIA_TYPE_VIDEO
        RuntimeHostMediaAssetKind.Audio -> MediaStore.Files.FileColumns.MEDIA_TYPE_AUDIO
        RuntimeHostMediaAssetKind.Other -> MediaStore.Files.FileColumns.MEDIA_TYPE_NONE
    }
}

private fun runtimeMediaKind(
    kind: Int,
): RuntimeHostMediaAssetKind {
    return when (kind) {
        MediaStore.Files.FileColumns.MEDIA_TYPE_IMAGE -> RuntimeHostMediaAssetKind.Image
        MediaStore.Files.FileColumns.MEDIA_TYPE_VIDEO -> RuntimeHostMediaAssetKind.Video
        MediaStore.Files.FileColumns.MEDIA_TYPE_AUDIO -> RuntimeHostMediaAssetKind.Audio
        else -> RuntimeHostMediaAssetKind.Other
    }
}

private fun android.database.Cursor.getStringOrNull(
    columnName: String,
): String? {
    val columnIndex = getColumnIndex(columnName)
    if (columnIndex == -1 || isNull(columnIndex)) {
        return null
    }

    return getString(columnIndex)
}

private fun android.database.Cursor.getIntOrDefault(
    columnName: String,
): Int {
    val columnIndex = getColumnIndex(columnName)
    if (columnIndex == -1 || isNull(columnIndex)) {
        return 0
    }

    return getInt(columnIndex)
}

private fun android.database.Cursor.getLongOrDefault(
    columnName: String,
): Long {
    val columnIndex = getColumnIndex(columnName)
    if (columnIndex == -1 || isNull(columnIndex)) {
        return 0L
    }

    return getLong(columnIndex)
}
