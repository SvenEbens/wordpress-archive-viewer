use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter};
use crate::config::AppConfig;
use crate::errors::{AppError, Result};
use crate::port::find_multiple_free_ports;

pub mod mariadb;
pub mod php;
pub mod caddy;

#[derive(Default)]
pub struct RuntimeState {
    pub mariadb_process: Option<tokio::process::Child>,
    pub php_process: Option<tokio::process::Child>,
    pub caddy_process: Option<tokio::process::Child>,
    pub db_port: u16,
    pub php_port: u16,
    pub http_port: u16,
}

pub type SharedRuntimeState = Arc<Mutex<RuntimeState>>;

#[derive(serde::Serialize, Clone)]
struct StatusPayload {
    service: String,
    status: String,
}

#[derive(serde::Serialize, Clone)]
pub struct SiteInfo {
    pub name: String,
    pub url: String,
    pub wordpress_version: Option<String>,
    pub php_version: Option<String>,
    pub port: u16,
    pub created: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct AllReadyPayload {
    pub status: String,
    pub site_info: SiteInfo,
}

pub async fn start_all(
    app: &AppHandle,
    config: &AppConfig,
    state: SharedRuntimeState,
    doc_root: &std::path::Path,
) -> Result<(u16, u16, u16)> {
    let mut st = state.lock().await;

    // 1. Allocate Ports
    let ports = find_multiple_free_ports(3)?;
    st.db_port = ports[0];
    st.php_port = ports[1];
    st.http_port = ports[2];

    tracing::info!("Allocated ports: DB={}, PHP={}, HTTP={}", st.db_port, st.php_port, st.http_port);

    // 2. Start MariaDB
    app.emit("status_update", StatusPayload { service: "db".into(), status: "starting".into() }).unwrap();
    st.mariadb_process = Some(mariadb::start(config, st.db_port).await?);
    mariadb::wait_until_ready(st.db_port).await?;
    app.emit("status_update", StatusPayload { service: "db".into(), status: "running".into() }).unwrap();

    // 3. Start PHP-CGI
    app.emit("status_update", StatusPayload { service: "php".into(), status: "starting".into() }).unwrap();
    st.php_process = Some(php::start(config, st.php_port).await?);
    php::wait_until_ready(st.php_port).await?;
    app.emit("status_update", StatusPayload { service: "php".into(), status: "running".into() }).unwrap();

    // 4. Start Caddy
    app.emit("status_update", StatusPayload { service: "server".into(), status: "starting".into() }).unwrap();
    st.caddy_process = Some(caddy::start(config, doc_root, st.http_port, st.php_port).await?);
    caddy::wait_until_ready(st.http_port).await?;
    app.emit("status_update", StatusPayload { service: "server".into(), status: "running".into() }).unwrap();

    Ok((st.db_port, st.php_port, st.http_port))
}

pub async fn stop_all(state: SharedRuntimeState) {
    let mut st = state.lock().await;

    // Stop Caddy
    if let Some(mut proc) = st.caddy_process.take() {
        tracing::info!("Stopping Caddy...");
        let _ = proc.kill().await;
    }

    // Stop PHP
    if let Some(mut proc) = st.php_process.take() {
        tracing::info!("Stopping PHP-CGI...");
        let _ = proc.kill().await;
    }

    // Stop MariaDB
    if let Some(mut proc) = st.mariadb_process.take() {
        tracing::info!("Stopping MariaDB...");
        let _ = proc.kill().await;
    }
}
