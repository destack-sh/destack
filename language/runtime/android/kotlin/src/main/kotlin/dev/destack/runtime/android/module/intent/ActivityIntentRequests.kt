package dev.destack.runtime.android.module.intent

import android.content.ClipData
import android.content.Intent
import android.net.Uri

import androidx.activity.ComponentActivity
import androidx.core.content.FileProvider
import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk

import java.io.File
import java.net.URLConnection

private const val fileProviderSuffix: String = ".destack.runtime.fileprovider"

/**
 * The Android intent request surface backed by one attached activity.
 */
public class ActivityIntentRequests(
    private val activity: ComponentActivity,
) : IntentRequests {
    override fun canOpenUrl(
        url: String,
    ): Boolean {
        val parsedUrl = parseUri(url) ?: return false
        val intent = Intent(Intent.ACTION_VIEW, parsedUrl)

        return intent.resolveActivity(activity.packageManager) != null
    }

    override fun openUrl(
        url: String,
    ): Int {
        val parsedUrl = parseUri(url) ?: return hostStatusInvalidArgument
        val intent = Intent(Intent.ACTION_VIEW, parsedUrl)

        return launchIntent(activity, intent)
    }

    override fun openPath(
        path: String,
    ): Int {
        val file = File(path)
        if (!file.exists()) {
            return hostStatusInvalidArgument
        }

        val uri = fileUri(activity, file)
        val mimeType = URLConnection.guessContentTypeFromName(file.name) ?: "*/*"
        val intent = Intent(Intent.ACTION_VIEW)
            .setDataAndType(uri, mimeType)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)

        return launchIntent(activity, intent)
    }

    override fun shareText(
        text: String,
        contentType: String?,
    ): Int {
        val intent = Intent(Intent.ACTION_SEND)
            .setType(contentType ?: "text/plain")
            .putExtra(Intent.EXTRA_TEXT, text)

        return launchChooser(activity, intent)
    }

    override fun sharePaths(
        paths: List<String>,
        contentType: String?,
    ): Int {
        if (paths.isEmpty()) {
            return hostStatusInvalidArgument
        }

        val files = paths.map(::File)
        if (files.any { !it.exists() }) {
            return hostStatusInvalidArgument
        }

        val uris = files.map { fileUri(activity, it) }
        val mimeType = contentType ?: inferredContentType(files)
        val intent = if (uris.size == 1) {
            Intent(Intent.ACTION_SEND)
                .putExtra(Intent.EXTRA_STREAM, uris.first())
        } else {
            Intent(Intent.ACTION_SEND_MULTIPLE)
                .putParcelableArrayListExtra(Intent.EXTRA_STREAM, ArrayList(uris))
        }

        intent.type = mimeType
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        intent.clipData = clipData(activity, uris)

        return launchChooser(activity, intent)
    }
}

private fun parseUri(
    value: String,
): Uri? {
    return try {
        Uri.parse(value)
    }
    catch (_: Exception) {
        null
    }
}

private fun fileUri(
    activity: ComponentActivity,
    file: File,
): Uri {
    return FileProvider.getUriForFile(
        activity,
        activity.packageName + fileProviderSuffix,
        file,
    )
}

private fun launchIntent(
    activity: ComponentActivity,
    intent: Intent,
): Int {
    return try {
        if (intent.resolveActivity(activity.packageManager) == null) {
            return hostStatusNotSupported
        }

        activity.startActivity(intent)
        hostStatusOk
    }
    catch (_: IllegalArgumentException) {
        hostStatusInvalidArgument
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

private fun launchChooser(
    activity: ComponentActivity,
    intent: Intent,
): Int {
    return try {
        val chooser = Intent.createChooser(intent, null)
        chooser.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)

        activity.startActivity(chooser)
        hostStatusOk
    }
    catch (_: IllegalArgumentException) {
        hostStatusInvalidArgument
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

private fun inferredContentType(
    files: List<File>,
): String {
    val mimeTypes = files
        .map { URLConnection.guessContentTypeFromName(it.name) ?: "*/*" }
        .distinct()

    return if (mimeTypes.size == 1) mimeTypes.first() else "*/*"
}

private fun clipData(
    activity: ComponentActivity,
    uris: List<Uri>,
): ClipData {
    val firstUri = uris.first()
    val clipData = ClipData.newUri(activity.contentResolver, null, firstUri)

    for (uri in uris.drop(1)) {
        clipData.addItem(ClipData.Item(uri))
    }

    return clipData
}
