import { useState } from "react";
import { api } from "../lib/api";
import type { Instance } from "../App";

type Hit = {
  id: string;
  slug: string;
  title: string;
  description: string;
  downloads: number;
  source: string;
  project_type: string;
};

type Props = {
  instance?: Instance;
  instances: Instance[];
  setToast: (s: string) => void;
};

export default function StorePage({ instance, instances, setToast }: Props) {
  const [tab, setTab] = useState<"mod" | "shader" | "modpack">("mod");
  const [source, setSource] = useState("modrinth");
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<Hit[]>([]);
  const [instId, setInstId] = useState(instance?.id ?? "");
  const [mrpackPath, setMrpackPath] = useState("");

  const search = async () => {
    try {
      const list = await api<Hit[]>("store_search", {
        query,
        projectType: tab,
        source,
      });
      setHits(list);
    } catch (e) {
      setToast(String(e));
    }
  };

  const install = async (projectId: string) => {
    const target = instId || instance?.id;
    if (!target) {
      setToast("请选择实例");
      return;
    }
    try {
      const versions = await api<Array<{ id: string }>>("store_versions", {
        projectId,
      });
      const vid = versions[0]?.id;
      if (!vid) {
        setToast("无可用版本");
        return;
      }
      const dest = tab === "shader" ? "shader" : "mod";
      const path = await api<string>("store_install", {
        instanceId: target,
        versionId: vid,
        destKind: dest,
      });
      setToast(`已安装到 ${path}`);
    } catch (e) {
      setToast(String(e));
    }
  };

  return (
    <section className="panel">
      <h2>内容商店</h2>
      <p className="muted">模组 · 光影 · 整合包（Modrinth / CurseForge）</p>
      <div className="tabs">
        {(["mod", "shader", "modpack"] as const).map((t) => (
          <button key={t} className={tab === t ? "active" : ""} onClick={() => setTab(t)}>
            {t === "mod" ? "模组" : t === "shader" ? "光影" : "整合包"}
          </button>
        ))}
      </div>
      <div className="row" style={{ marginBottom: "0.8rem" }}>
        <input
          placeholder="搜索（支持中文关键词）"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select value={source} onChange={(e) => setSource(e.target.value)} style={{ maxWidth: 140 }}>
          <option value="modrinth">Modrinth</option>
          <option value="curseforge">CurseForge</option>
        </select>
        <select
          value={instId || instance?.id || ""}
          onChange={(e) => setInstId(e.target.value)}
          style={{ maxWidth: 200 }}
        >
          <option value="">安装到实例</option>
          {instances.map((i) => (
            <option key={i.id} value={i.id}>
              {i.name}
            </option>
          ))}
        </select>
        <button onClick={search}>搜索</button>
      </div>

      <div className="list">
        {hits.map((h) => (
          <div className="list-item" key={`${h.source}-${h.id}`}>
            <div>
              <strong>{h.title}</strong>
              <div className="muted">
                {h.source} · {h.downloads} downloads
              </div>
              <div className="muted">{h.description}</div>
            </div>
            {tab !== "modpack" && (
              <button onClick={() => install(h.id)}>安装</button>
            )}
          </div>
        ))}
      </div>

      <h3 style={{ marginTop: "1.4rem" }}>导入 mrpack</h3>
      <div className="row">
        <input
          placeholder="本地 .mrpack 路径"
          value={mrpackPath}
          onChange={(e) => setMrpackPath(e.target.value)}
        />
        <button
          onClick={async () => {
            const target = instId || instance?.id;
            if (!target) {
              setToast("请选择实例");
              return;
            }
            try {
              const report = await api<{ name: string; downloaded: number; skipped: number }>(
                "import_mrpack_cmd",
                { instanceId: target, path: mrpackPath },
              );
              setToast(
                `导入 ${report.name}: 下载 ${report.downloaded}, 跳过 ${report.skipped}`,
              );
            } catch (e) {
              setToast(String(e));
            }
          }}
        >
          导入
        </button>
      </div>
    </section>
  );
}
