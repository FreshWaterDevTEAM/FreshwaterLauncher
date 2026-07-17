# FWL Sync 协议

服主发布客户端 Mod 清单，玩家在启动器绑定后一键同步。

## 端点

```
GET {base}/v1/channels/{channel}/manifest.json
GET {base}/files/mods/...
```

## Manifest

```json
{
  "protocol": 1,
  "channel": "default",
  "revision": 3,
  "mc": "1.20.1",
  "loader": "fabric",
  "loader_version": "0.15.0",
  "files": [
    {
      "path": "mods/example.jar",
      "sha256": "...",
      "size": 12345,
      "url": "https://sync.example.com/files/mods/example.jar"
    }
  ],
  "remove": [],
  "rules": { "strict_mods": true }
}
```

- `revision` 必须单调递增  
- `strict_mods=true` 时删除清单未列出的 `mods/` 文件  
- 默认建议先「检查更新」预览 diff，再一键同步  

## 服主 5 分钟部署

```bash
cargo run -p fwl-sync-server -- publish \
  --instance ./aligned-client \
  --out ./publish \
  --channel default \
  --revision 1 \
  --mc 1.20.1 \
  --public-url http://YOUR_IP:8787

cargo run -p fwl-sync-server -- serve --root ./publish --bind 0.0.0.0:8787
```

也可只把 `publish/` 丢到 Nginx/OSS 做静态托管。

玩家填写：`http://YOUR_IP:8787` 或邀请码 `fwl://sync?base=...&channel=default`。
