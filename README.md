# FreshwaterLauncher (FWL)

跨平台 Minecraft **Java Edition** 启动器：Windows / macOS / Linux / Android（ARM）。

- 自研实现（不基于 PCL 源码）
- 微软 Device Code 正版登录 / 离线 / Authlib-Injector
- 版本下载、Fabric/Quilt/Forge、国内镜像
- 内容商店（Mod / 光影 / 整合包）
- **FWL Sync**：服主自建更新源，玩家一键同步客户端 Mod
- 兼容标准 `.minecraft` 与 Modrinth `.mrpack`

仓库：<https://github.com/FreshWaterDevTEAM/FreshwaterLauncher>

## 开发

### 依赖

- Node.js 20+
- Rust stable
- **Windows**：需安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（勾选「使用 C++ 的桌面开发」），提供 `link.exe`
- 桌面：各平台 Tauri 系统依赖  
- Android：Android SDK / NDK（`npm run tauri android init` 后开发）

### 命令

```bash
npm install
npm run tauri:dev
```

构建：

```bash
npm run tauri:build
```

服主同步服务：

```bash
cargo run -p fwl-sync-server -- --help
```

## 配置

- Azure Client ID 默认已写入（公共客户端）：见 [docs/microsoft-auth.md](docs/microsoft-auth.md)
- CurseForge：环境变量 `FWL_CURSEFORGE_API_KEY`

## 文档

- [微软登录](docs/microsoft-auth.md)
- [数据目录](docs/data-layout.md)
- [功能清单](docs/feature-parity.md)
- [FWL Sync](docs/fwl-sync-protocol.md)

## License

MIT
