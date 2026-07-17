# Android（ARM）说明

## 当前状态

- Tauri 2 Android：`src-tauri/tauri.android.conf.json`，包名 `com.freshwater.fwl`
- **CI** 构建 `FreshwaterLauncher-android-arm64.apk`（`tauri android init` → `scripts/patch-android-project.sh` → release APK）
- **Runtime**：从 `android-runtime/index.json` 下载 Android OpenJDK 17（`.tar.xz`），解压到数据目录 `runtimes/android-<abi>/jre`
- **出游**：`FwlGameActivity` 优先桥接已安装的 **PojavLauncher / FCL**；否则用外置 `java` 进程启动（GLES 栈仍不完整，裸 JRE 可能无法进入画面）
- UI：窄屏底栏；商店 / Sync / 账号与桌面共用 `fwl-core`

## 本地构建

```bash
npm install
# 需要 Android SDK / NDK / JDK 17
npx tauri android init
chmod +x scripts/patch-android-project.sh
./scripts/patch-android-project.sh
npx tauri android dev
# 或
npx tauri android build --apk --target aarch64
```

## 使用流程（手机）

1. 安装 APK，登录账号，下载版本并创建实例（与桌面相同数据布局）
2. 打开「更多」→ **下载 Android Runtime**（首次约数百 MB）
3. 首页点启动：
   - 若已装 Pojav / FCL：会拉起对方并提示游戏目录
   - 否则写入 `fwl-android-launch.json` 并由 `FwlGameActivity` 执行 `java`

## 出游层说明

真正稳定进游戏需要 JVM + GLES/输入桥。FWL 刻意 **不** 把 GPL 的 Pojav 链进本仓库（MIT），而是：

1. 外置进程执行社区 Android JRE
2. 可选 Intent 桥到用户已安装的 Pojav / FCL

桌面侧 `fwl-android-runtime` 在非 Android 目标上可编译（probe / prepare）；下载解压逻辑仅在 `target_os=android` 启用。
