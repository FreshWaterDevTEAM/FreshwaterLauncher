import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppConfig, Instance } from "../App";

type Props = {
  config: AppConfig | null;
  instance?: Instance;
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
};

type ServerEntry = { name: string; address: string };

export default function MorePage({ config, instance, onRefresh, setToast }: Props) {
  const [mem, setMem] = useState(config?.max_memory_mb ?? 4096);
  const [java, setJava] = useState(config?.java_path ?? "");
  const [jvm, setJvm] = useState(config?.jvm_args ?? "");
  const [servers, setServers] = useState<ServerEntry[]>([]);
  const [sName, setSName] = useState("");
  const [sAddr, setSAddr] = useState("");
  const [log, setLog] = useState("");
  const [skin, setSkin] = useState("");
  const [androidStatus, setAndroidStatus] = useState("");
  const [androidBusy, setAndroidBusy] = useState(false);

  useEffect(() => {
    if (config) {
      setMem(config.max_memory_mb);
      setJava(config.java_path ?? "");
      setJvm(config.jvm_args);
    }
  }, [config]);

  const refreshAndroid = async () => {
    try {
      const st = await api<{
        ready: boolean;
        message: string;
        abi: string;
        java_home?: string | null;
      }>("android_runtime_status");
      setAndroidStatus(
        st.ready
          ? `就绪 (${st.abi}) ${st.java_home ?? ""}`
          : `${st.message} [${st.abi}]`,
      );
    } catch (e) {
      setAndroidStatus(String(e));
    }
  };

  useEffect(() => {
    (async () => {
      if (!instance) return;
      try {
        setServers(await api("read_servers_dat", { instanceId: instance.id }));
        setSkin(await api("skin_preview_url", { uuid: "Steve" }));
        await refreshAndroid();
      } catch {
        /* ignore */
      }
    })();
  }, [instance]);

  return (
    <div className="grid-2">
      <section className="panel">
        <h2>设置</h2>
        <label className="muted">最大内存 (MB)</label>
        <input
          type="number"
          value={mem}
          onChange={(e) => setMem(Number(e.target.value))}
        />
        <label className="muted" style={{ display: "block", marginTop: "0.6rem" }}>
          Java 路径
        </label>
        <input value={java} onChange={(e) => setJava(e.target.value)} />
        <label className="muted" style={{ display: "block", marginTop: "0.6rem" }}>
          JVM 参数
        </label>
        <input value={jvm} onChange={(e) => setJvm(e.target.value)} />
        <div className="row" style={{ marginTop: "0.8rem" }}>
          <button
            onClick={async () => {
              if (!config) return;
              await api("save_config", {
                config: {
                  ...config,
                  max_memory_mb: mem,
                  java_path: java || null,
                  jvm_args: jvm,
                },
              });
              await onRefresh();
              setToast("设置已保存");
            }}
          >
            保存
          </button>
          <button
            className="ghost"
            onClick={async () => {
              const list = await api<Array<{ path: string; version: string; major: number }>>(
                "list_java",
              );
              setToast(
                list.map((j) => `${j.major}: ${j.path}`).join("\n") || "未检测到 Java",
              );
            }}
          >
            探测 Java
          </button>
        </div>
        <p className="muted" style={{ marginTop: "1rem" }}>
          数据目录：{config?.data_dir}
        </p>
        <p className="muted">Client ID：{config?.ms_client_id}</p>
      </section>

      <div>
        <section className="panel">
          <h2>服务器列表</h2>
          <div className="row">
            <input placeholder="名称" value={sName} onChange={(e) => setSName(e.target.value)} />
            <input
              placeholder="地址 host:port"
              value={sAddr}
              onChange={(e) => setSAddr(e.target.value)}
            />
            <button
              onClick={async () => {
                const next = [...servers, { name: sName, address: sAddr }];
                await api("save_servers", { servers: next });
                setServers(next);
                setSName("");
                setSAddr("");
              }}
            >
              添加
            </button>
          </div>
          <div className="list" style={{ marginTop: "0.6rem" }}>
            {servers.map((s, idx) => (
              <div className="list-item" key={`${s.address}-${idx}`}>
                <div>
                  <strong>{s.name}</strong>
                  <div className="muted">{s.address}</div>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="panel">
          <h2>日志 / 崩溃</h2>
          <div className="row">
            <button
              className="ghost"
              disabled={!instance}
              onClick={async () => {
                if (!instance) return;
                setLog((await api("latest_log", { instanceId: instance.id })) || "无日志");
              }}
            >
              最新日志
            </button>
            <button
              className="ghost"
              disabled={!instance}
              onClick={async () => {
                if (!instance) return;
                setLog(
                  (await api("crash_summary", { instanceId: instance.id })) || "无崩溃报告",
                );
              }}
            >
              崩溃摘要
            </button>
          </div>
          {log && <pre className="toast" style={{ maxHeight: 220, overflow: "auto" }}>{log}</pre>}
        </section>

        <section className="panel">
          <h2>关于 FWL</h2>
          <p>
            FreshwaterLauncher 是自研开源跨端 Minecraft Java 版启动器，功能对标主流启动器体验，UI
            与实现均独立，不基于 PCL 源码。
          </p>
          <p className="muted">
            仓库：https://github.com/FreshWaterDevTEAM/FreshwaterLauncher
          </p>
          {skin && (
            <p className="muted">皮肤预览服务已接入（mc-heads）。</p>
          )}
          <p className="muted">Android Runtime：{androidStatus || "未探测"}</p>
          <div className="row" style={{ marginTop: "0.6rem" }}>
            <button className="ghost" onClick={() => refreshAndroid()}>
              探测 Runtime
            </button>
            <button
              disabled={androidBusy}
              onClick={async () => {
                setAndroidBusy(true);
                try {
                  const st = await api<{ ready: boolean; message: string }>(
                    "android_ensure_runtime",
                  );
                  setToast(st.ready ? "Android Runtime 已就绪" : st.message);
                  await refreshAndroid();
                } catch (e) {
                  setToast(String(e));
                } finally {
                  setAndroidBusy(false);
                }
              }}
            >
              {androidBusy ? "下载中…" : "下载 Android Runtime"}
            </button>
          </div>
        </section>
      </div>
    </div>
  );
}
