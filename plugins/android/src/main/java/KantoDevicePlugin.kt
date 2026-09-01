package ac.kanto.launcher.device

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import androidx.core.content.FileProvider
import java.io.File

@InvokeArg
class InstallArgs {
    lateinit var path: String
}

@TauriPlugin
class KantoDevicePlugin(private val activity: Activity): Plugin(activity) {
    @Command
    fun capabilities(invoke: Invoke) {
        val response = JSObject()
        response.put("supports32BitApps", Build.SUPPORTED_32_BIT_ABIS.isNotEmpty())
        invoke.resolve(response)
    }

    @Command
    fun install(invoke: Invoke) {
        val args = invoke.parseArgs(InstallArgs::class.java)
        val file = File(args.path)
        if (!file.isFile) {
            invoke.reject("prepared APK is missing")
            return
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
            !activity.packageManager.canRequestPackageInstalls()
        ) {
            activity.startActivity(Intent(
                Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:${activity.packageName}"),
            ))
            val response = JSObject()
            response.put("started", false)
            response.put("needsPermission", true)
            invoke.resolve(response)
            return
        }

        val uri = FileProvider.getUriForFile(
            activity,
            "${activity.packageName}.fileprovider",
            file,
        )
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, "application/vnd.android.package-archive")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        activity.startActivity(intent)
        val response = JSObject()
        response.put("started", true)
        response.put("needsPermission", false)
        invoke.resolve(response)
    }
}
