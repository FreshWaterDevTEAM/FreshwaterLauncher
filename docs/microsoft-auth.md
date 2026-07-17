# Microsoft 正版登录（FWL）

## Azure 应用

- **应用程序（客户端）ID**：`9e27bb17-91c2-49ce-b7b4-d667665e82da`
- 类型：公共客户端（无 Client Secret）
- 必须开启：**允许公共客户端流 = 是**
- 登录租户：`consumers`
- Scope：`XboxLive.signin offline_access`
- 默认流程：**OAuth 2.0 Device Code**

可用环境变量覆盖：

```bash
FWL_MS_CLIENT_ID=your-client-id
```

## Minecraft API 审核

新应用需提交：<https://aka.ms/mce-reviewappid>

未通过时 `api.minecraftservices.com` 可能返回 **403**。审核期间请用离线账号或 Authlib 测试启动链路。

### 理由模板（English）

```
FreshwaterLauncher (FWL) is an open-source, third-party Minecraft: Java Edition launcher for Windows, macOS, Linux, and Android. We need Minecraft Services API access so players can sign in with their personal Microsoft accounts (OAuth 2.0 device code / Xbox Live → Minecraft Services), verify game ownership, obtain profiles, and launch the game legally.
We do not collect or sell account credentials. Tokens are stored only on the user’s device for session refresh. The project is open source at: https://github.com/FreshWaterDevTEAM/FreshwaterLauncher
Azure Application (client) ID: 9e27bb17-91c2-49ce-b7b4-d667665e82da
```

## 登录链路

1. Device Code 授权 → Microsoft access / refresh token  
2. Xbox Live authenticate  
3. XSTS（RelyingParty = `rp://api.minecraftservices.com/`）  
4. `login_with_xbox`  
5. entitlements + profile  

令牌保存在用户数据目录 `accounts.json`。
