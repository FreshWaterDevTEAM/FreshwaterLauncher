use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use clap::{Parser, Subcommand};
use fwl_core::sync::{
    build_manifest_from_instance, copy_instance_mods_to_publish, invite_code, SyncManifest,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Parser)]
#[command(name = "fwl-sync-server", about = "FWL Sync — 服主自建客户端 Mod 更新服务")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 从已对齐的客户端实例目录生成并发布清单
    Publish {
        /// 实例游戏目录（含 mods/）
        #[arg(long)]
        instance: PathBuf,
        /// 输出发布根目录
        #[arg(long)]
        out: PathBuf,
        /// 频道名
        #[arg(long, default_value = "default")]
        channel: String,
        /// revision（单调递增）
        #[arg(long)]
        revision: u64,
        /// MC 版本号
        #[arg(long)]
        mc: String,
        /// 玩家访问的公开根 URL，如 https://sync.example.com
        #[arg(long)]
        public_url: String,
    },
    /// 启动 HTTP 服务托管发布目录
    Serve {
        #[arg(long, default_value = "./publish")]
        root: PathBuf,
        #[arg(long, default_value = "0.0.0.0:8787")]
        bind: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Publish {
            instance,
            out,
            channel,
            revision,
            mc,
            public_url,
        } => {
            std::fs::create_dir_all(&out)?;
            copy_instance_mods_to_publish(&instance, &out)?;
            let manifest = build_manifest_from_instance(
                &instance,
                &channel,
                revision,
                &mc,
                &public_url,
            )?;
            let channel_dir = out.join("v1").join("channels").join(&channel);
            std::fs::create_dir_all(&channel_dir)?;
            let path = channel_dir.join("manifest.json");
            std::fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
            println!("Published revision {} to {}", revision, path.display());
            println!("Players bind: {}", public_url);
            println!("Invite: {}", invite_code(&public_url, &channel));
        }
        Commands::Serve { root, bind } => {
            let addr: SocketAddr = bind.parse()?;
            let app = Router::new()
                .route(
                    "/v1/channels/{channel}/manifest.json",
                    get(manifest_handler),
                )
                .nest_service("/files", ServeDir::new(root.join("files")))
                .layer(CorsLayer::permissive())
                .with_state(root);
            println!("FWL Sync listening on http://{addr}");
            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}

async fn manifest_handler(
    axum::extract::State(root): axum::extract::State<PathBuf>,
    Path(channel): Path<String>,
) -> impl IntoResponse {
    let path = root
        .join("v1")
        .join("channels")
        .join(&channel)
        .join("manifest.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<SyncManifest>(&text) {
            Ok(m) => (StatusCode::OK, Json(m)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(_) => (StatusCode::NOT_FOUND, "manifest not found").into_response(),
    }
}
