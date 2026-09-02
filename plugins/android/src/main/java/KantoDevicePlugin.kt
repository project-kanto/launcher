package ac.kanto.launcher.device

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.content.pm.PackageManager
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import androidx.core.content.FileProvider
import java.io.File
import java.security.MessageDigest

@InvokeArg
class InstallArgs {
    lateinit var path: String
}

@InvokeArg
class GameArgs {
    lateinit var packageName: String
}

@TauriPlugin
class KantoDevicePlugin(private val activity: Activity): Plugin(activity) {
    @Command
    fun capabilities(invoke: Invoke) {
        val args = invoke.parseArgs(GameArgs::class.java)
        Thread {
            val response = JSObject()
            response.put("supports32BitApps", Build.SUPPORTED_32_BIT_ABIS.isNotEmpty())
            response.put("canInstallApps", canInstallApps())
            installedGame(args.packageName)?.let { response.put("installedGame", it) }
            activity.runOnUiThread { invoke.resolve(response) }
        }.start()
    }

    @Command
    fun install(invoke: Invoke) {
        val args = invoke.parseArgs(InstallArgs::class.java)
        val file = File(args.path)
        if (!file.isFile) {
            invoke.reject("prepared APK is missing")
            return
        }

        if (!canInstallApps()) {
            openInstallSettings()
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

    @Command
    fun openGame(invoke: Invoke) {
        val args = invoke.parseArgs(GameArgs::class.java)
        val intent = activity.packageManager.getLaunchIntentForPackage(args.packageName)
        if (intent == null) {
            invoke.reject("Kanto is not installed")
            return
        }
        activity.startActivity(intent)
        val response = JSObject()
        response.put("opened", true)
        invoke.resolve(response)
    }

    @Command
    fun openInstallSettings(invoke: Invoke) {
        openInstallSettings()
        val response = JSObject()
        response.put("opened", true)
        invoke.resolve(response)
    }

    private fun openInstallSettings() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            activity.startActivity(Intent(
                Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:${activity.packageName}"),
            ))
        } else {
            activity.startActivity(Intent(Settings.ACTION_SECURITY_SETTINGS))
        }
    }

    @Suppress("DEPRECATION")
    private fun canInstallApps(): Boolean =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            activity.packageManager.canRequestPackageInstalls()
        } else {
            Settings.Global.getInt(
                activity.contentResolver,
                Settings.Global.INSTALL_NON_MARKET_APPS,
                0,
            ) == 1
        }

    @Suppress("DEPRECATION")
    private fun installedGame(packageName: String): JSObject? {
        val info = try {
            activity.packageManager.getPackageInfo(packageName, 0)
        } catch (_: PackageManager.NameNotFoundException) {
            return null
        }
        val response = JSObject()
        response.put("packageName", packageName)
        response.put("versionName", info.versionName)
        info.applicationInfo?.sourceDir?.let { response.put("sha256", sha256(File(it))) }
        return response
    }

    private fun sha256(file: File): String? = try {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(64 * 1024)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        digest.digest().joinToString("") { "%02x".format(it.toInt() and 0xff) }
    } catch (_: Exception) {
        null
    }
}
