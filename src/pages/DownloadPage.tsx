import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppConfig } from "../App";

type VersionInfo = {
  id: string;
  type: string;
  url: string;
};

type Props = {
  config: AppConfig | null;
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
};

export default function DownloadPage({ config, onRefresh, setToast }: Props) {
  const [versions, setVersions] = useState<VersionInfo[]>([]);
  const [filter, setFilter] = useState("release");
  const [mc, setMc] = useState("1.20.1");
  const [loader, setLoader] = useState("fabric");
  const [tasks, setTasks] = useState<
    { id: string; name: string; status: string; progress: number; message: string }[]
  >([]);

  const load = async () => {
    try {
      const m = await api<{ versions: VersionInfo[] }>("fetch_versions");
      setVersions(m.versions || []);
    } catch (e) {
      setToast(String(e));
    }
  };

  useEffect(() => {
    load();
    const t = setInterval(async () => {
      try {
        setTasks(await api("download_tasks"));
      } catch {
        /* ignore */
      }
    }, 1500);
    return () => clearInterval(t);
  }, []);

  const filtered = versions.filter((v) =>
    filter === "all" ? true : (v.type || (v as { version_type?: string }).version_type) === filter,
  ).slice(0, 80);

  return (
    <div className="grid-2">
      <section className="panel">
        <h2>游戏版本</h2>
        <div className="row" style={{ marginBottom: "0.8rem" }}>
          <select value={filter} onChange={(e) => setFilter(e.target.value)} style={{ maxWidth: 160 }}>
            <option value="release">正式版</option>
            <option value="snapshot">快照</option>
            <option value="all">全部</option>
          </select>
          <button
            className="ghost"
            onClick={async () => {
              await api("set_download_source", {
                source: config?.download_source === "official" ? "bmclapi" : "official",
              });
              await onRefresh();
              await load();
              setToast("已切换下载源");
            }}
          >
            源: {config?.download_source || "bmclapi"}
          </button>
          <button className="ghost" onClick={load}>
            刷新列表
          </button>
        </div>
        <div className="list">
          {filtered.map((v) => (
            <div className="list-item" key={v.id}>
              <div>
                <strong>{v.id}</strong>
                <div className="muted">{v.type}</div>
              </div>
              <button
                onClick={async () => {
                  try {
                    setToast(`开始安装 ${v.id}…`);
                    await api("install_version", { versionId: v.id });
                    await api("create_instance", {
                      name: v.id,
                      versionId: v.id,
                    });
                    setToast(`已安装并创建实例 ${v.id}`);
                    await onRefresh();
                  } catch (e) {
                    setToast(String(e));
                  }
                }}
              >
                安装
              </button>
            </div>
          ))}
        </div>
      </section>

      <div>
        <section className="panel">
          <h2>加载器</h2>
          <p className="muted">Fabric / Quilt 一键配置；Forge 下载安装器；OptiFine 打开官网。</p>
          <div className="row">
            <input value={mc} onChange={(e) => setMc(e.target.value)} placeholder="MC 版本" />
            <select value={loader} onChange={(e) => setLoader(e.target.value)} style={{ maxWidth: 140 }}>
              <option value="fabric">Fabric</option>
              <option value="quilt">Quilt</option>
              <option value="forge">Forge</option>
              <option value="neoforge">NeoForge</option>
              <option value="optifine">OptiFine</option>
            </select>
            <button
              onClick={async () => {
                try {
                  const id = await api<string>("install_loader", {
                    kind: loader,
                    mcVersion: mc,
                  });
                  setToast(`加载器结果: ${id}`);
                  if (loader === "fabric" || loader === "quilt") {
                    await api("create_instance", { name: id, versionId: id });
                    await onRefresh();
                  }
                } catch (e) {
                  setToast(String(e));
                }
              }}
            >
              安装加载器
            </button>
          </div>
        </section>

        <section className="panel">
          <h2>下载任务</h2>
          <div className="list">
            {tasks.length === 0 && <p className="muted">暂无任务</p>}
            {tasks.map((t) => (
              <div className="list-item" key={t.id}>
                <div>
                  <strong>{t.name}</strong>
                  <div className="muted">
                    {t.status} · {Math.round(t.progress * 100)}% · {t.message}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
