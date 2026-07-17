import { useState } from "react";
import { api } from "../lib/api";
import type { Account, AppConfig, Instance } from "../App";

type Props = {
  config: AppConfig | null;
  account?: Account;
  instance?: Instance;
  accounts: Account[];
  instances: Instance[];
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
};

export default function HomePage({
  config,
  account,
  instance,
  accounts,
  instances,
  onRefresh,
  setToast,
}: Props) {
  const [busy, setBusy] = useState(false);
  const [accId, setAccId] = useState(account?.id ?? "");
  const [instId, setInstId] = useState(instance?.id ?? "");

  const launch = async () => {
    const a = accId || account?.id;
    const i = instId || instance?.id;
    if (!a || !i) {
      setToast("请先选择账号和实例");
      return;
    }
    setBusy(true);
    try {
      if (config) {
        await api("save_config", {
          config: { ...config, selected_account: a, selected_instance: i },
        });
      }
      const pid = await api<number>("launch_instance", {
        instanceId: i,
        accountId: a,
      });
      setToast(pid === 0 ? "已拉起 Android 出游界面" : `已启动游戏 (pid ${pid})`);
      await onRefresh();
    } catch (e) {
      setToast(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="panel hero-launch">
      <p className="muted" style={{ marginBottom: 0 }}>
        FreshwaterLauncher
      </p>
      <h1>清水一启，即刻出发</h1>
      <p className="muted">
        自研跨端 Java 版启动器 · 兼容标准目录与 mrpack · 支持服主一键同步
      </p>
      <div className="row" style={{ marginTop: "1.2rem" }}>
        <select
          style={{ maxWidth: 220 }}
          value={accId || account?.id || ""}
          onChange={(e) => setAccId(e.target.value)}
        >
          <option value="">选择账号</option>
          {accounts.map((a) => (
            <option key={a.id} value={a.id}>
              {a.username} ({a.kind})
            </option>
          ))}
        </select>
        <select
          style={{ maxWidth: 260 }}
          value={instId || instance?.id || ""}
          onChange={(e) => setInstId(e.target.value)}
        >
          <option value="">选择实例</option>
          {instances.map((i) => (
            <option key={i.id} value={i.id}>
              {i.name} · {i.version_id}
            </option>
          ))}
        </select>
        <button disabled={busy} onClick={launch}>
          {busy ? "启动中…" : "启动游戏"}
        </button>
      </div>
    </section>
  );
}
