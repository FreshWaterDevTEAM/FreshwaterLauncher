# Android（ARM）说明

## 当前状态

- Tauri 2 Android 配置：`src-tauri/tauri.android.conf.json`
- 包名：`com.freshwater.fwl`
- `fwl-android-runtime`：LaunchBackend 抽象 + Stub（桌面可编译）
- UI：窄屏自动切换底栏导航，商店 / Sync / 账号流程与桌面共用 `fwl-core`
- **CI**：Release 工作流会构建 `FreshwaterLauncher-android-arm64.apk`

## 本地初始化

```bash
npm install
# 需要 Android SDK / NDK / JDK 17
npx tauri android init
npx tauri android dev
# 或 release APK
npx tauri android build --apk --target aarch64
```

## 出游层

真正在手机上跑 Java Edition 需要 JVM + GLES/输入桥（工作量大）。  
`AndroidLaunchRequest` 已与桌面实例目录对齐；后续在 `fwl-android-runtime` 接入兼容许可证的原生桥即可，不必改动商店/Sync/下载逻辑。

当前 APK 可安装启动器壳（账号/下载/商店/Sync UI）；进游戏能力随 MobileRuntime 完善。
