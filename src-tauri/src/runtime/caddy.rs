use std::path::Path;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use reqwest;
use crate::config::AppConfig;
use crate::errors::{AppError, Result};

pub async fn start(config: &AppConfig, doc_root: &Path, http_port: u16, php_port: u16) -> Result<tokio::process::Child> {
    let caddy_dir = config.runtime_dir.join("caddy");
    let caddy_exe = caddy_dir.join(format!("caddy{}", std::env::consts::EXE_SUFFIX));

    if !caddy_exe.exists() {
        return Err(AppError::Runtime("Caddy not found. Did you run setup-runtime.ps1?".into()));
    }

    // Kill any leftover caddy processes from previous sessions
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "caddy.exe"])
            .creation_flags(0x08000000)
            .output()
            .await;
        sleep(Duration::from_millis(300)).await;
    }
    
    #[cfg(unix)]
    {
        let _ = Command::new("killall")
            .arg("caddy")
            .output()
            .await;
        sleep(Duration::from_millis(300)).await;
    }

    // Generate Caddyfile
    let caddyfile_path = caddy_dir.join("Caddyfile");
    let doc_root_str = doc_root.display().to_string().replace('\\', "/");
    
    let caddyfile_content = format!(
        ":{} {{\n\
            root * \"{}\"\n\
            php_fastcgi 127.0.0.1:{}\n\
            file_server\n\
            encode gzip\n\
        }}\n",
        http_port, doc_root_str, php_port
    );
    std::fs::write(&caddyfile_path, caddyfile_content)?;

    tracing::info!("Starting Caddy on port {}", http_port);
    let mut cmd = Command::new(&caddy_exe);
    cmd.arg("run")
       .arg("--config")
       .arg(&caddyfile_path)
       .current_dir(&caddy_dir)
       .kill_on_drop(true);
       
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    
    let child = cmd.spawn()?;

    Ok(child)
}

pub async fn wait_until_ready(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let mut retries = 0;
    
    while retries < 20 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
        retries += 1;
    }
    Err(AppError::Runtime("Caddy failed to start within timeout".into()))
}
