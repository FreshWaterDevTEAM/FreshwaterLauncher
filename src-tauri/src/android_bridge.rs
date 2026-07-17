//! Start FwlGameActivity from Rust via JNI.

#[cfg(target_os = "android")]
pub fn open_game_activity(launch_file: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;

    let activity = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
    let class = env
        .find_class("com/freshwater/fwl/FwlNative")
        .map_err(|e| format!("FwlNative class: {e}"))?;
    let file = env
        .new_string(launch_file)
        .map_err(|e| format!("jstring: {e}"))?;

    env.call_static_method(
        class,
        "startGame",
        "(Landroid/content/Context;Ljava/lang/String;)V",
        &[JValue::Object(&activity), JValue::Object(&file)],
    )
    .map_err(|e| format!("startGame: {e}"))?;

    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn open_game_activity(_launch_file: &str) -> Result<(), String> {
    Err("open_game_activity is Android-only".into())
}
