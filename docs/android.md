# Android（ARM）说明

## 架构

FWL Android APK **内嵌** Amethyst/Pojav 出游内核（LGPL-3.0，见 `NOTICE.android`）：

1. Tauri UI + Rust 准备 `fwl-android-launch.json`
2. JNI → `FwlGameActivity` 映射目录 / 账号 / Runtime
3. 拉起同 APK 内的 `net.kdt.pojavlaunch.MainActivity` 出游

**不做**外置 Pojav/FCL Intent 桥接。

## 构建要点

`scripts/patch-android-project.sh` 会：

- 将 Amethyst 暂存为 library 模块 `:app_pojavlauncher`
- 写入与上游一致的 AGP 开关（否则 Java 编不过）：
  - `android.nonFinalResIds=false`（`switch(R.id.*)`）
  - `android.nonTransitiveRClass=false`（portrait-sdp / gamepad / AppCompat 资源）
- 为 library 补 `BuildConfig.VERSION_NAME`
- JitPack + `libbytehook.so` pickFirst

## 本地构建

```bash
git submodule update --init --recursive
git -C third_party/amethyst-android submodule update --init --recursive
npm install
npx tauri android init
chmod +x scripts/patch-android-project.sh
./scripts/patch-android-project.sh
npx tauri android build --apk --target aarch64
```

## 手机使用

1. 安装 FWL APK（内含内核）
2. 下载版本 / 创建实例 /（可选）在「更多」下载 Android Runtime
3. 首页启动 → 同进程内核出游
