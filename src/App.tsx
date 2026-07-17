import { useEffect, useMemo, useState } from "react";
import { api } from "./lib/api";
import HomePage from "./pages/HomePage";
import AccountsPage from "./pages/AccountsPage";
import DownloadPage from "./pages/DownloadPage";
import InstancesPage from "./pages/InstancesPage";
import StorePage from "./pages/StorePage";
import SyncPage from "./pages/SyncPage";
import MorePage from "./pages/MorePage";

export type NavKey =
  | "home"
  | "accounts"
  | "download"
  | "instances"
  | "store"
  | "sync"
  | "more";

export type Account = {
  id: string;
  kind: string;
  username: string;
  uuid: string;
};

export type Instance = {
  id: string;
  name: string;
  version_id: string;
  sync_platform?: string | null;
};

export type AppConfig = {
  data_dir: string;
  ms_client_id: string;
  max_memory_mb: number;
  min_memory_mb: number;
  java_path?: string | null;
  jvm_args: string;
  download_source: string;
  selected_instance?: string | null;
  selected_account?: string | null;
};

const NAV: { key: NavKey; label: string }[] = [
  { key: "home", label: "首页" },
  { key: "download", label: "下载" },
  { key: "instances", label: "实例" },
  { key: "store", label: "商店" },
  { key: "sync", label: "服主同步" },
  { key: "accounts", label: "账号" },
  { key: "more", label: "更多" },
];

export default function App() {
  const [nav, setNav] = useState<NavKey>("home");
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [instances, setInstances] = useState<Instance[]>([]);
  const [toast, setToast] = useState<string>("");

  const refresh = async () => {
    try {
      const [c, a, i] = await Promise.all([
        api<AppConfig>("get_config"),
        api<Account[]>("list_accounts"),
        api<Instance[]>("list_instances"),
      ]);
      setConfig(c);
      setAccounts(a);
      setInstances(i);
    } catch (e) {
      setToast(String(e));
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const selectedAccount = useMemo(
    () => accounts.find((a) => a.id === config?.selected_account) ?? accounts[0],
    [accounts, config],
  );
  const selectedInstance = useMemo(
    () => instances.find((i) => i.id === config?.selected_instance) ?? instances[0],
    [instances, config],
  );

  return (
    <div className="app-shell">
      <aside className="nav">
        <div className="brand">
          <p className="brand-mark">FWL</p>
          <p className="brand-sub">FreshwaterLauncher</p>
        </div>
        {NAV.map((item) => (
          <button
            key={item.key}
            className={nav === item.key ? "active" : ""}
            onClick={() => setNav(item.key)}
          >
            {item.label}
          </button>
        ))}
      </aside>

      <main className="content">
        {nav === "home" && (
          <HomePage
            config={config}
            account={selectedAccount}
            instance={selectedInstance}
            accounts={accounts}
            instances={instances}
            onRefresh={refresh}
            setToast={setToast}
          />
        )}
        {nav === "accounts" && (
          <AccountsPage accounts={accounts} onRefresh={refresh} setToast={setToast} />
        )}
        {nav === "download" && (
          <DownloadPage config={config} onRefresh={refresh} setToast={setToast} />
        )}
        {nav === "instances" && (
          <InstancesPage
            instances={instances}
            onRefresh={refresh}
            setToast={setToast}
          />
        )}
        {nav === "store" && (
          <StorePage
            instance={selectedInstance}
            instances={instances}
            setToast={setToast}
          />
        )}
        {nav === "sync" && (
          <SyncPage instances={instances} onRefresh={refresh} setToast={setToast} />
        )}
        {nav === "more" && (
          <MorePage
            config={config}
            instance={selectedInstance}
            onRefresh={refresh}
            setToast={setToast}
          />
        )}
        {toast && <div className="toast">{toast}</div>}
      </main>

      <nav className="mobile-nav">
        {NAV.slice(0, 5).map((item) => (
          <button
            key={item.key}
            className={nav === item.key ? "active" : ""}
            onClick={() => setNav(item.key)}
          >
            {item.label}
          </button>
        ))}
      </nav>
    </div>
  );
}
