package com.freshwater.fwl

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import net.kdt.pojavlaunch.MainActivity
import net.kdt.pojavlaunch.Tools
import net.kdt.pojavlaunch.prefs.LauncherPreferences
import net.kdt.pojavlaunch.value.MinecraftAccount
import org.json.JSONObject
import java.io.File
import java.nio.file.Files
import java.util.concurrent.Executors

/**
 * Bridges FWL launch.json into the embedded Amethyst/Pojav [MainActivity] kernel.
 */
class FwlGameActivity : Activity() {
    private val tag = "FWL-Game"
    private val executor = Executors.newSingleThreadExecutor()
    private lateinit var logView: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        logView = TextView(this).apply {
            setTextIsSelectable(true)
            textSize = 12f
            setPadding(24, 24, 24, 24)
        }
        setContentView(
            ScrollView(this).apply {
                addView(
                    LinearLayout(this@FwlGameActivity).apply {
                        orientation = LinearLayout.VERTICAL
                        addView(logView)
                    },
                )
            },
        )

        val launchPath = intent.getStringExtra(EXTRA_LAUNCH_FILE)
        if (launchPath.isNullOrBlank()) {
            appendLog("缺少 launch 文件路径")
            return
        }
        executor.execute { prepareAndLaunch(launchPath) }
    }

    override fun onDestroy() {
        executor.shutdownNow()
        super.onDestroy()
    }

    private fun prepareAndLaunch(launchPath: String) {
        try {
            val json = JSONObject(File(launchPath).readText())
            val versionId = json.optString("version_id")
            val gameDir = json.optString("game_dir")
            val username = json.optString("username", "Player")
            val uuid = json.optString("uuid", "00000000-0000-0000-0000-000000000000")
            val accessToken = json.optString("access_token", "0")
            val assetsDir = json.optString("assets_dir")
            val librariesDir = json.optString("libraries_dir")
            val javaHome = json.optString("java_home", "")

            appendLog("初始化 Amethyst/Pojav 存储…")
            Tools.initEarlyConstants(this)
            if (!Tools.checkStorageRoot(this)) {
                appendLog("存储不可用，无法出游")
                return
            }
            Tools.initStorageConstants(this)
            LauncherPreferences.loadPreferences(this)

            val mcRoot = File(Tools.DIR_GAME_NEW)
            mcRoot.mkdirs()

            // Map FWL shared dirs into Pojav .minecraft layout (symlink when possible).
            linkOrCopyDir(File(librariesDir), File(mcRoot, "libraries"))
            linkOrCopyDir(File(assetsDir), File(mcRoot, "assets"))
            val fwlVersions = File(gameDir).parentFile?.parentFile?.resolve("versions")
                ?: File(librariesDir).parentFile?.resolve("versions")
            if (fwlVersions != null && fwlVersions.exists()) {
                linkOrCopyDir(fwlVersions, File(mcRoot, "versions"))
            } else {
                // Instance-local version jar/json may live under shared versions by id
                appendLog("versions 目录: ${File(mcRoot, "versions").absolutePath}")
            }

            // Point profile gameDir at FWL instance folder so mods/saves stay with the instance
            val profiles = File(mcRoot, "launcher_profiles.json")
            if (!profiles.exists()) {
                profiles.writeText(
                    """
                    {
                      "profiles": {
                        "FWL": {
                          "name": "FWL",
                          "type": "custom",
                          "lastVersionId": "$versionId",
                          "gameDir": "$gameDir"
                        }
                      },
                      "selectedProfile": "FWL"
                    }
                    """.trimIndent(),
                )
            }

            val account = MinecraftAccount()
            account.username = username
            account.accessToken = accessToken
            account.profileId = uuid
            account.selectedVersion = versionId
            account.isMicrosoft = accessToken.isNotBlank() && accessToken != "0"
            try {
                File(Tools.DIR_ACCOUNT_NEW).mkdirs()
                account.save()
            } catch (t: Throwable) {
                appendLog("写入账号信息跳过: ${t.message}")
            }

            if (javaHome.isNotBlank()) {
                tryInstallRuntime(File(javaHome))
            }

            if (versionId.isBlank()) {
                appendLog("launch.json 缺少 version_id")
                return
            }

            appendLog("启动内核 MainActivity：$versionId")
            val i = Intent(this, MainActivity::class.java)
            i.putExtra(MainActivity.INTENT_MINECRAFT_VERSION, versionId)
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
            startActivity(i)
            finish()
        } catch (t: Throwable) {
            Log.e(tag, "prepareAndLaunch failed", t)
            appendLog("出游失败: ${t.message}")
        }
    }

    private fun tryInstallRuntime(jreHome: File) {
        try {
            val marker = File(jreHome, ".fwl-jre-root")
            val root = if (marker.exists()) File(marker.readText().trim()) else jreHome
            if (!File(root, "bin/java").exists() && !File(root, "bin/java").canExecute()) {
                // still try
            }
            appendLog("注册 Runtime: ${root.absolutePath}")
            val dest = File(Tools.MULTIRT_HOME, "Internal-17")
            if (!dest.exists()) {
                linkOrCopyDir(root, dest)
            }
        } catch (t: Throwable) {
            appendLog("Runtime 注册警告: ${t.message}")
        }
    }

    private fun linkOrCopyDir(src: File, dest: File) {
        if (!src.exists()) {
            appendLog("跳过缺失目录: ${src.absolutePath}")
            return
        }
        if (dest.exists()) {
            return
        }
        dest.parentFile?.mkdirs()
        try {
            Files.createSymbolicLink(dest.toPath(), src.toPath())
            appendLog("symlink ${dest.name} → ${src.absolutePath}")
            return
        } catch (_: Throwable) {
            /* fall through */
        }
        try {
            src.copyRecursively(dest, overwrite = false)
            appendLog("copied ${dest.name}")
        } catch (t: Throwable) {
            appendLog("无法映射 ${src.name}: ${t.message}")
        }
    }

    private fun appendLog(msg: String) {
        Log.i(tag, msg)
        runOnUiThread {
            logView.append(msg)
            logView.append("\n")
        }
    }

    companion object {
        const val EXTRA_LAUNCH_FILE = "fwl_launch_file"
    }
}
