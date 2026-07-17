import { useState } from "react";
import { api } from "../lib/api";
import type { Instance } from "../App";

type Props = {
  instances: Instance[];
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
};

type LocalMod = { file_name: string; enabled: boolean; path: string };

export default function InstancesPage({ instances, onRefresh, setToast }: Props) {
  const [selected, setSelected] = useState<string>("");
  const [mods, setMods] = useState<LocalMod[]>([]);
  const [shaders, setShaders] = useState<string[]>([]);
  const [name, setName] = useState("");
  const [versionId, setVersionId] = useState("");
  const [installed, setInstalled] = useState<string[]>([]);

  const pick = async (id: string) => {
    setSelected(id);
    try {
      setMods(await api("get_mods", { instanceId: id }));
      setShaders(await api("get_shaderpacks", { instanceId: id }));
    } catch (e) {
      setToast(String(e));
    }
  };

  return (
    <div className="grid-2">
      <section className="panel">
        <h2>实例</h2>
        <div className="row" style={{ marginBottom: "0.8rem" }}>
          <input placeholder="名称" value={name} onChange={(e) => setName(e.target.value)} />
          <input
            placeholder="版本 ID"
            value={versionId}
            onChange={(e) => setVersionId(e.target.value)}
          />
          <button
            onClick={async () => {
              try {
                await api("create_instance", { name, versionId });
                setName("");
                await onRefresh();
              } catch (e) {
                setToast(String(e));
              }
            }}
          >
            创建
          </button>
          <button
            className="ghost"
            onClick={async () => {
              setInstalled(await api("scanned_versions"));
            }}
          >
            扫描已安装版本
          </button>
        </div>
        {installed.length > 0 && (
          <p className="muted">已安装: {installed.join(", ")}</p>
        )}
        <div className="list">
          {instances.map((i) => (
            <div
              key={i.id}
              className={`list-item ${selected === i.id ? "active" : ""}`}
              onClick={() => pick(i.id)}
            >
              <div>
                <strong>{i.name}</strong>
                <div className="muted">
                  {i.version_id}
                  {i.sync_platform ? " · 已绑 Sync" : ""}
                </div>
              </div>
              <button
                className="danger"
                onClick={async (e) => {
                  e.stopPropagation();
                  await api("delete_instance", { id: i.id, deleteFiles: true });
                  await onRefresh();
                }}
              >
                删除
              </button>
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <h2>内容管理</h2>
        {!selected && <p className="muted">选择一个实例</p>}
        {selected && (
          <>
            <h3>Mods</h3>
            <div className="list">
              {mods.map((m) => (
                <div className="list-item" key={m.path}>
                  <span>{m.file_name}</span>
                  <button
                    className="ghost"
                    onClick={async () => {
                      await api("toggle_mod", {
                        path: m.path,
                        enabled: !m.enabled,
                      });
                      pick(selected);
                    }}
                  >
                    {m.enabled ? "禁用" : "启用"}
                  </button>
                </div>
              ))}
            </div>
            <h3 style={{ marginTop: "1rem" }}>光影</h3>
            <div className="list">
              {shaders.length === 0 && <p className="muted">无光影包</p>}
              {shaders.map((s) => (
                <div className="list-item" key={s}>
                  {s}
                </div>
              ))}
            </div>
            <div className="row" style={{ marginTop: "1rem" }}>
              <button
                className="ghost"
                onClick={async () => {
                  try {
                    await api("export_mrpack_cmd", {
                      instanceId: selected,
                      path: `${selected}.mrpack`,
                    });
                    setToast("已导出 mrpack（当前工作目录）");
                  } catch (e) {
                    setToast(String(e));
                  }
                }}
              >
                导出 mrpack
              </button>
            </div>
          </>
        )}
      </section>
    </div>
  );
}
