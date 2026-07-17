import { useState } from "react";
import { api } from "../lib/api";
import type { Instance } from "../App";

type Props = {
  instances: Instance[];
  onRefresh: () => Promise<void>;
  setToast: (s: string) => void;
};

export default function SyncPage({ instances, onRefresh, setToast }: Props) {
  const [instId, setInstId] = useState("");
  const [platform, setPlatform] = useState("");
  const [diff, setDiff] = useState<string>("");

  return (
    <div className="grid-2">
      <section className="panel">
        <h2>服务器平台（玩家）</h2>
        <p className="muted">
          填写服主提供的 Sync URL 或邀请码，一键对齐客户端 Mod。
        </p>
        <select value={instId} onChange={(e) => setInstId(e.target.value)}>
          <option value="">选择实例</option>
          {instances.map((i) => (
            <option key={i.id} value={i.id}>
              {i.name}
              {i.sync_platform ? "（已绑定）" : ""}
            </option>
          ))}
        </select>
        <input
          style={{ marginTop: "0.6rem" }}
          placeholder="https://sync.example.com 或 fwl://sync?..."
          value={platform}
          onChange={(e) => setPlatform(e.target.value)}
        />
        <div className="row" style={{ marginTop: "0.8rem" }}>
          <button
            onClick={async () => {
              try {
                await api("bind_sync_platform", {
                  instanceId: instId,
                  platform,
                });
                await onRefresh();
                setToast("已绑定服务器平台");
              } catch (e) {
                setToast(String(e));
              }
            }}
          >
            绑定
          </button>
          <button
            className="ghost"
            onClick={async () => {
              try {
                const data = await api<{
                  diff: {
                    revision: number;
                    up_to_date: boolean;
                    to_download: { path: string }[];
                    to_remove: string[];
                  };
                  localRevision?: number;
                }>("sync_check", { instanceId: instId });
                setDiff(
                  `远端 revision ${data.diff.revision} / 本地 ${data.localRevision ?? "-"}\n` +
                    `需下载 ${data.diff.to_download.length} · 需删除 ${data.diff.to_remove.length}\n` +
                    data.diff.to_download.map((f) => `+ ${f.path}`).join("\n") +
                    (data.diff.to_remove.length
                      ? "\n" + data.diff.to_remove.map((r) => `- ${r}`).join("\n")
                      : ""),
                );
                setToast(data.diff.up_to_date ? "已是最新" : "发现更新");
              } catch (e) {
                setToast(String(e));
              }
            }}
          >
            检查更新
          </button>
          <button
            onClick={async () => {
              try {
                const report = await api<{
                  downloaded: string[];
                  removed: string[];
                  revision: number;
                }>("sync_apply", { instanceId: instId });
                setToast(
                  `同步完成 rev ${report.revision}: +${report.downloaded.length} -${report.removed.length}`,
                );
              } catch (e) {
                setToast(String(e));
              }
            }}
          >
            一键同步
          </button>
        </div>
        {diff && <div className="toast">{diff}</div>}
      </section>

      <section className="panel">
        <h2>服主工具</h2>
        <p className="muted">
          使用仓库内 <code>fwl-sync-server</code>：
        </p>
        <pre
          style={{
            whiteSpace: "pre-wrap",
            background: "rgba(7,24,28,0.55)",
            padding: "0.9rem",
            borderRadius: 12,
            fontSize: "0.85rem",
          }}
        >{`# 从已对齐客户端实例发布
cargo run -p fwl-sync-server -- publish \\
  --instance ./my-client-instance \\
  --out ./publish \\
  --channel default \\
  --revision 1 \\
  --mc 1.20.1 \\
  --public-url https://sync.example.com

# 托管
cargo run -p fwl-sync-server -- serve --root ./publish --bind 0.0.0.0:8787`}</pre>
        <InviteMaker setToast={setToast} />
      </section>
    </div>
  );
}

function InviteMaker({ setToast }: { setToast: (s: string) => void }) {
  const [base, setBase] = useState("http://127.0.0.1:8787");
  const [channel, setChannel] = useState("default");
  return (
    <div style={{ marginTop: "1rem" }}>
      <h3>生成邀请码</h3>
      <div className="row">
        <input value={base} onChange={(e) => setBase(e.target.value)} />
        <input
          style={{ maxWidth: 140 }}
          value={channel}
          onChange={(e) => setChannel(e.target.value)}
        />
        <button
          className="ghost"
          onClick={async () => {
            const code = await api<string>("make_invite", { base, channel });
            setToast(code);
          }}
        >
          生成
        </button>
      </div>
    </div>
  );
}
