package com.freshwater.fwl

import android.app.Activity
import android.content.Context
import android.content.Intent

/** JNI entry for Rust → Kotlin game start. */
object FwlNative {
    @JvmStatic
    fun startGame(context: Context, launchFile: String) {
        val intent = Intent(context, FwlGameActivity::class.java)
        intent.putExtra(FwlGameActivity.EXTRA_LAUNCH_FILE, launchFile)
        if (context !is Activity) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        context.startActivity(intent)
    }
}
