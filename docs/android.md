# Android（ARM）说明

## 当前状态

- Tauri 2 Android：`src-tauri/tauri.android.conf.json`，包名 `com.freshwater.fwl`
- **内嵌出游内核**：[Amethyst-Android](https://github.com/AngelAuraMC/Amethyst-Android)（Pojav 谱系，LGPL-3.0）submodule 于 `third_party/amethyst-android`
- CI：`tauri android init` → `scripts/patch-android-project.sh`（库化 Amethyst + 注入依赖）→ release APK
- **许可**：桌面/共享代码 MIT；Android 出游栈见 [NOTICE.android](../NOTICE.android) 与根 [LICENSE](../LICENSE) 例外说明
- UI：账号 / 下载 / 实例 / 商店 / Sync 仍走 `fwl-core`；点启动后进入 Amethyst `MainActivity`

## 本地构建

```bash
git submodule update --init --recursive
# Amethyst 内层 submodule（SDL 等）
git -C third_party/amethyst-android submodule update --init --recursive

npm install
# Android SDK / NDK / JDK 17
npx tauri android init
chmod +x scripts/patch-android-project.sh
./scripts/patch-android-project.sh
npx tauri android build --apk --target aarch64
```

## 使用流程（手机）

1. 安装 APK，登录账号，下载版本并创建实例
2. 「更多」→ **下载 Android Runtime**（MultiRT / JRE17）
3. 首页启动 → `FwlGameActivity` 映射目录并拉起内嵌 `MainActivity`

## 目录映射

FWL 数据目录中的 `libraries` / `assets` / `versions` 会链接或复制到 Pojav 的 `.minecraft`；实例 `game_dir` 写入 `launcher_profiles.json`，模组与存档仍跟实例走。
