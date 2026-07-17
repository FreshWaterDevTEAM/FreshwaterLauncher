# 数据目录与兼容约定

默认数据根目录：

- Windows: `%APPDATA%/FreshwaterLauncher`
- macOS: `~/Library/Application Support/FreshwaterLauncher`
- Linux: `~/.local/share/FreshwaterLauncher`
- Android: 应用私有存储下的同名结构

## 布局

```
FreshwaterLauncher/
  config.toml.json
  accounts.json
  instances.json
  servers.json
  java/
  logs/
  .minecraft/
    versions/<id>/<id>.json|jar
    libraries/
    assets/indexes|objects
  instances/<name>/
    mods/
    resourcepacks/
    shaderpacks/
    saves/
    config/
    natives/
    .fwl-sync.json
```

该 `.minecraft` 布局与主流启动器（含 PCL 用户目录）兼容：可将 `versions/libraries/assets` 指向或复制到已有目录以复用下载。

## 整合包

- 导入：Modrinth `.mrpack`（`modrinth.index.json` + `overrides/`）
- 导出：生成基础 mrpack（overrides 含 mods）
