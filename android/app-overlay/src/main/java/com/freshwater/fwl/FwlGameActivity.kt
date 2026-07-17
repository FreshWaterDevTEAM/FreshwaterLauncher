package com.freshwater.fwl

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import org.json.JSONObject
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader
import java.util.concurrent.Executors

/**
 * Starts Minecraft: Java Edition using a downloaded Android JRE as an external process.
 * Also offers bridge intents to PojavLauncher / FCL when installed.
 */
class FwlGameActivity : Activity() {
    private val tag = "FWL-Game"
    private val executor = Executors.newSingleThreadExecutor()
    private lateinit var logView: TextView
    private var process: Process? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        logView = TextView(this).apply {
            setTextIsSelectable(true)
            textSize = 12f
            setPadding(24, 24, 24, 24)
        }
        val scroll = ScrollView(this).apply { addView(logView) }
        setContentView(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                addView(
                    scroll,
                    LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.MATCH_PARENT,
                    ),
                )
            },
        )

        val launchPath = intent.getStringExtra(EXTRA_LAUNCH_FILE)
        if (launchPath.isNullOrBlank()) {
            appendLog("缺少 launch 文件路径")
            return
        }
        executor.execute { startGame(launchPath) }
    }

    override fun onDestroy() {
        process?.destroy()
        executor.shutdownNow()
        super.onDestroy()
    }

    private fun startGame(launchPath: String) {
        try {
            val json = JSONObject(File(launchPath).readText())
            val gameDir = json.getString("game_dir")
            val javaHome = json.optString("java_home", "")
            val mainClass = json.optString("main_class", "net.minecraft.client.main.Main")
            val classpath = json.optJSONArray("classpath") ?: org.json.JSONArray()
            val jvmArgs = json.optJSONArray("jvm_args") ?: org.json.JSONArray()
            val gameArgs = json.optJSONArray("game_args") ?: org.json.JSONArray()
            val natives = json.optString("natives_dir", "")

            if (tryBridgeExternal(gameDir, json)) {
                return
            }

            val javaBin = resolveJava(javaHome)
            if (javaBin == null) {
                appendLog("未找到 Android JRE。请先在「更多」页下载 Android Runtime。")
                appendLog("也可安装 PojavLauncher / Fold Craft Launcher，FWL 会自动桥接出游。")
                return
            }

            val cp = buildList {
                for (i in 0 until classpath.length()) add(classpath.getString(i))
            }.joinToString(":")

            val cmd = ArrayList<String>()
            cmd.add(javaBin.absolutePath)
            for (i in 0 until jvmArgs.length()) {
                val a = jvmArgs.getString(i)
                if (a.startsWith("-Djava.library.path=")) continue
                cmd.add(a)
            }
            if (natives.isNotBlank()) {
                cmd.add("-Djava.library.path=$natives")
            }
            cmd.add("-Dorg.lwjgl.opengl.libname=libgl4es_32.so")
            cmd.add("-cp")
            cmd.add(cp)
            cmd.add(mainClass)
            for (i in 0 until gameArgs.length()) cmd.add(gameArgs.getString(i))

            appendLog("启动: ${cmd.take(6).joinToString(" ")} ...")
            val pb = ProcessBuilder(cmd)
                .directory(File(gameDir))
                .redirectErrorStream(true)
            val env = pb.environment()
            env["JAVA_HOME"] = javaHome
            env["LIBGL_ES"] = "2"
            env["LIBGL_MIPMAP"] = "3"
            env["MESA_GL_VERSION_OVERRIDE"] = "3.3"
            if (natives.isNotBlank()) {
                val prev = env["LD_LIBRARY_PATH"] ?: ""
                env["LD_LIBRARY_PATH"] = listOf(natives, "$javaHome/lib", prev)
                    .filter { it.isNotBlank() }
                    .joinToString(":")
            }

            process = pb.start()
            val reader = BufferedReader(InputStreamReader(process!!.inputStream))
            var line: String?
            while (reader.readLine().also { line = it } != null) {
                appendLog(line!!)
            }
            val code = process!!.waitFor()
            appendLog("进程退出: $code")
        } catch (t: Throwable) {
            Log.e(tag, "launch failed", t)
            appendLog("启动失败: ${t.message}")
        }
    }

    private fun tryBridgeExternal(gameDir: String, json: JSONObject): Boolean {
        val candidates = listOf(
            "com.tungsten.fcl",
            "com.tungsten.fclauncher",
            "net.kdt.pojavlaunch",
            "net.kdt.pojavlaunch.debug",
        )
        for (pkg in candidates) {
            val launch = packageManager.getLaunchIntentForPackage(pkg) ?: continue
            appendLog("检测到 $pkg，使用外部运行时出游（推荐）")
            launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            launch.putExtra("FWL_GAME_DIR", gameDir)
            launch.putExtra("FWL_VERSION_ID", json.optString("version_id"))
            launch.putExtra("FWL_USERNAME", json.optString("username"))
            try {
                startActivity(launch)
                appendLog("已拉起 $pkg。请在该启动器中选择对应版本/目录继续。")
                appendLog("游戏目录: $gameDir")
                return true
            } catch (t: Throwable) {
                appendLog("无法拉起 $pkg: ${t.message}")
            }
        }
        return false
    }

    private fun resolveJava(javaHome: String): File? {
        if (javaHome.isBlank()) return null
        val home = File(javaHome)
        val marker = File(home, ".fwl-jre-root")
        val root = if (marker.exists()) File(marker.readText().trim()) else home
        val candidates = listOf(
            File(root, "bin/java"),
            File(root, "jre/bin/java"),
            File(home, "bin/java"),
        )
        candidates.firstOrNull { it.exists() && it.canExecute() }?.let { return it }
        home.walkTopDown().maxDepth(3).forEach { f ->
            if (f.name == "java" && f.canExecute() && f.parentFile?.name == "bin") {
                return f
            }
        }
        return null
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

        fun open(activity: Activity, launchFile: String) {
            val i = Intent(activity, FwlGameActivity::class.java)
            i.putExtra(EXTRA_LAUNCH_FILE, launchFile)
            activity.startActivity(i)
        }
    }
}
