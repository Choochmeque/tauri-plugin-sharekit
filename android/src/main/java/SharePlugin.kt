package app.tauri.share

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.webkit.WebView
import android.net.Uri
import android.webkit.MimeTypeMap
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import androidx.core.content.FileProvider
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.util.UUID

@InvokeArg
class ShareTextOptions {
    lateinit var text: String
    var mimeType: String = "text/plain"
    var title: String? = null
}

@InvokeArg
class ShareFileOptions {
    lateinit var url: String
    var mimeType: String = "*/*"
    var title: String? = null
}

@TauriPlugin
class SharePlugin(private val activity: Activity): Plugin(activity) {
    private var pendingSharedContent: JSObject? = null

    override fun load(webView: WebView) {
        super.load(webView)
        handleIntent(activity.intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        if (intent == null) return

        when (intent.action) {
            Intent.ACTION_SEND -> handleSingleShare(intent)
            Intent.ACTION_SEND_MULTIPLE -> handleMultipleShare(intent)
        }

        if (pendingSharedContent != null) {
            trigger("sharedContent", pendingSharedContent!!)
        }
    }

    private fun handleSingleShare(intent: Intent) {
        val type = intent.type ?: return

        if (type.startsWith("text/")) {
            val text = intent.getStringExtra(Intent.EXTRA_TEXT)
            if (text != null) {
                pendingSharedContent = JSObject().apply {
                    put("type", "text")
                    put("text", text)
                }
            }
        } else {
            val uri = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
            } else {
                @Suppress("DEPRECATION")
                intent.getParcelableExtra(Intent.EXTRA_STREAM)
            }

            if (uri != null) {
                val file = copyUriToCache(uri)
                if (file != null) {
                    val filesArray = JSONArray()
                    filesArray.put(file)
                    pendingSharedContent = JSObject().apply {
                        put("type", "files")
                        put("files", filesArray)
                    }
                }
            }
        }
    }

    private fun handleMultipleShare(intent: Intent) {
        val uris = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
        } else {
            @Suppress("DEPRECATION")
            intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM)
        }

        if (uris != null && uris.isNotEmpty()) {
            val filesArray = JSONArray()
            for (uri in uris) {
                val file = copyUriToCache(uri)
                if (file != null) {
                    filesArray.put(file)
                }
            }
            if (filesArray.length() > 0) {
                pendingSharedContent = JSObject().apply {
                    put("type", "files")
                    put("files", filesArray)
                }
            }
        }
    }

    private fun copyUriToCache(uri: Uri): JSONObject? {
        return try {
            val contentResolver = activity.contentResolver
            val mimeType = contentResolver.getType(uri)

            var fileName = getFileName(uri)
            if (fileName == null) {
                val ext = MimeTypeMap.getSingleton().getExtensionFromMimeType(mimeType) ?: ""
                fileName = "${UUID.randomUUID()}.$ext"
            }

            val cacheDir = File(activity.cacheDir, "shared_files")
            cacheDir.mkdirs()
            val destFile = File(cacheDir, fileName)

            contentResolver.openInputStream(uri)?.use { input ->
                FileOutputStream(destFile).use { output ->
                    input.copyTo(output)
                }
            }

            JSONObject().apply {
                put("path", destFile.absolutePath)
                put("name", fileName)
                if (mimeType != null) put("mimeType", mimeType)
                put("size", destFile.length())
            }
        } catch (e: Exception) {
            null
        }
    }

    private fun getFileName(uri: Uri): String? {
        var result: String? = null
        if (uri.scheme == "content") {
            activity.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
                if (cursor.moveToFirst()) {
                    val nameIndex = cursor.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                    if (nameIndex >= 0) {
                        result = cursor.getString(nameIndex)
                    }
                }
            }
        }
        if (result == null) {
            result = uri.path?.substringAfterLast('/')
        }
        return result
    }

    @Command
    fun getPendingSharedContent(invoke: Invoke) {
        if (pendingSharedContent != null) {
            invoke.resolve(pendingSharedContent!!)
        } else {
            invoke.resolve()
        }
    }

    @Command
    fun clearPendingSharedContent(invoke: Invoke) {
        pendingSharedContent = null
        invoke.resolve()
    }

    /**
     * Open the native sharing interface to share some text
     */
    @Command
    fun shareText(invoke: Invoke) {
        val args = invoke.parseArgs(ShareTextOptions::class.java)

        val sendIntent = Intent().apply {
            this.action = Intent.ACTION_SEND
            this.type = args.mimeType
            this.putExtra(Intent.EXTRA_TEXT, args.text)
            this.putExtra(Intent.EXTRA_TITLE, args.title)
        }

        val shareIntent = Intent.createChooser(sendIntent, null)
        startActivityForResult(invoke, shareIntent, "shareTextResult")
    }

    @ActivityCallback
    private fun shareTextResult(invoke: Invoke, result: ActivityResult) {
        if (result.resultCode == Activity.RESULT_CANCELED) {
            invoke.reject("Share cancelled")
            return
        }
        invoke.resolve()
    }

    /**
     * Open the native sharing interface to share a file
     */
    @Command
    fun shareFile(invoke: Invoke) {
        val args = invoke.parseArgs(ShareFileOptions::class.java)
        
        // Get the source file from the URL
        val sourceFile = if (args.url.startsWith("file://")) {
            File(Uri.parse(args.url).path!!)
        } else {
            File(args.url)
        }
        
        // Create a temporary file to store the data
        val tempFile = File(activity.cacheDir, sourceFile.name)
        
        // Copy the source file to the temporary file
        sourceFile.inputStream().use { input ->
            tempFile.outputStream().use { output ->
                input.copyTo(output)
            }
        }

        // Get the authority from the app's manifest
        val authority = "${activity.packageName}.fileprovider"

        // Create a content URI for the file
        val contentUri = FileProvider.getUriForFile(activity, authority, tempFile)

        val sendIntent = Intent().apply {
            this.action = Intent.ACTION_SEND
            this.type = args.mimeType
            this.putExtra(Intent.EXTRA_STREAM, contentUri)
            this.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            this.putExtra(Intent.EXTRA_TITLE, args.title)
        }

        val shareIntent = Intent.createChooser(sendIntent, args.title)
        startActivityForResult(invoke, shareIntent, "shareFileResult")
    }

    @ActivityCallback
    private fun shareFileResult(invoke: Invoke, result: ActivityResult) {
        if (result.resultCode == Activity.RESULT_CANCELED) {
            invoke.reject("Share cancelled")
            return
        }
        invoke.resolve()
    }
}
