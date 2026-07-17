import { useState } from "react";
import { api } from "../lib/api";
import type { Account } from "../App";

type Props = {
  accounts: Account[];
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
};

export default function AccountsPage({ accounts, onRefresh, setToast }: Props) {
  const [offlineName, setOfflineName] = useState("");
  const [device, setDevice] = useState<{
    user_code: string;
    device_code: string;
    verification_uri: string;
    interval: number;
    message: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);

  const startMs = async () => {
    setBusy(true);
    try {
      const d = await api<typeof device extends infer T ? NonNullable<T> : never>(
        "ms_start_device_code",
      );
      setDevice(d);
      setToast(`${d.message || "请在浏览器完成登录"} 代码: ${d.user_code}`);
      // poll in background
      const acc = await api("ms_poll_device_code", {
        deviceCode: d.device_code,
        interval: d.interval || 5,
      });
      setToast(`微软账号登录成功: ${(acc as Account).username}`);
      setDevice(null);
      await onRefresh();
    } catch (e) {
      setToast(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addOffline = async () => {
    try {
      await api("add_offline", { username: offlineName });
      setOfflineName("");
      await onRefresh();
      setToast("已添加离线账号");
    } catch (e) {
      setToast(String(e));
    }
  };

  return (
    <div className="grid-2">
      <section className="panel">
        <h2>账号列表</h2>
        <div className="list">
          {accounts.length === 0 && <p className="muted">暂无账号</p>}
          {accounts.map((a) => (
            <div className="list-item" key={a.id}>
              <div>
                <strong>{a.username}</strong>
                <div className="muted">{a.kind}</div>
              </div>
              <div className="row">
                {a.kind === "microsoft" && (
                  <button
                    className="ghost"
                    onClick={async () => {
                      try {
                        await api("refresh_account", { id: a.id });
                        setToast("已刷新");
                        await onRefresh();
                      } catch (e) {
                        setToast(String(e));
                      }
                    }}
                  >
                    刷新
                  </button>
                )}
                <button
                  className="danger"
                  onClick={async () => {
                    await api("remove_account", { id: a.id });
                    await onRefresh();
                  }}
                >
                  删除
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <h2>微软正版登录</h2>
        <p className="muted">
          Device Code：打开微软页面输入代码。需 Azure Client ID 且 Minecraft API
          审核通过。
        </p>
        <button disabled={busy} onClick={startMs}>
          {busy ? "等待授权中…" : "微软设备码登录"}
        </button>
        {device && (
          <div className="toast ok" style={{ marginTop: "1rem" }}>
            验证地址：{device.verification_uri}
            {"\n"}代码：{device.user_code}
          </div>
        )}

        <h3 style={{ marginTop: "1.5rem" }}>离线账号</h3>
        <div className="row">
          <input
            placeholder="玩家名"
            value={offlineName}
            onChange={(e) => setOfflineName(e.target.value)}
          />
          <button onClick={addOffline}>添加</button>
        </div>

        <h3 style={{ marginTop: "1.5rem" }}>Authlib-Injector</h3>
        <AuthlibForm onRefresh={onRefresh} setToast={setToast} />
      </section>
    </div>
  );
}

function AuthlibForm({
  onRefresh,
  setToast,
}: {
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
}) {
  const [username, setUsername] = useState("");
  const [uuid, setUuid] = useState("");
  const [token, setToken] = useState("");
  const [server, setServer] = useState("");
  return (
    <div style={{ display: "grid", gap: "0.5rem" }}>
      <input placeholder="用户名" value={username} onChange={(e) => setUsername(e.target.value)} />
      <input placeholder="UUID" value={uuid} onChange={(e) => setUuid(e.target.value)} />
      <input placeholder="Access Token" value={token} onChange={(e) => setToken(e.target.value)} />
      <input
        placeholder="Authlib 服务器 URL"
        value={server}
        onChange={(e) => setServer(e.target.value)}
      />
      <button
        onClick={async () => {
          try {
            await api("add_authlib", {
              username,
              uuid,
              accessToken: token,
              server,
            });
            await onRefresh();
            setToast("已添加 Authlib 账号");
          } catch (e) {
            setToast(String(e));
          }
        }}
      >
        添加 Authlib 账号
      </button>
    </div>
  );
}
