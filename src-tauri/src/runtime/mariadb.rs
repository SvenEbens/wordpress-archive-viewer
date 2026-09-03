use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tokio::net::TcpStream;
use crate::config::AppConfig;
use crate::errors::{AppError, Result};

pub async fn start(config: &AppConfig, port: u16) -> Result<tokio::process::Child> {
    let maria_dir = config.runtime_dir.join("mariadb");
    let data_dir = maria_dir.join("data");
    let bin_dir = maria_dir.join("bin");

    if !maria_dir.exists() {
        return Err(AppError::Runtime("MariaDB runtime not found. Did you run setup-runtime.ps1?".into()));
    }

    // Kill any leftover mariadbd processes from previous sessions
    #[cfg(target_os = "windows")]
    {
        tracing::info!("Checking for stale MariaDB processes...");
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "mariadbd.exe"])
            .creation_flags(0x08000000)
            .output()
            .await;
        // Give the OS a moment to release the port
        sleep(Duration::from_millis(500)).await;
    }

    #[cfg(unix)]
    {
        tracing::info!("Checking for stale MariaDB processes...");
        let _ = Command::new("killall")
            .arg("mariadbd")
            .output()
            .await;
        // Give the OS a moment to release the port
        sleep(Duration::from_millis(500)).await;
    }

    // Clean up stale .pid files from previous crashed sessions
    if data_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&data_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("pid") {
                    tracing::info!("Removing stale PID file: {}", path.display());
                    let _ = std::fs::remove_file(&path);
                }
            }
        }

        // Remove conflicting my.ini inside data/ directory
        // (created by mariadb-install-db, conflicts with our dynamically generated one)
        let data_ini = data_dir.join("my.ini");
        if data_ini.exists() {
            tracing::info!("Removing conflicting data/my.ini");
            let _ = std::fs::remove_file(&data_ini);
        }
    }

    // Initialize data dir if it doesn't exist
    if !data_dir.exists() {
        tracing::info!("Initializing MariaDB data directory...");
        let mut cmd = Command::new(bin_dir.join(format!("mariadb-install-db{}", std::env::consts::EXE_SUFFIX)));
        cmd.arg(format!("--datadir={}", data_dir.display()))
           .current_dir(&maria_dir);
           
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        
        let status = cmd.status().await?;
            
        if !status.success() {
            return Err(AppError::Runtime("Failed to initialize MariaDB data directory".into()));
        }
    }

    // Generate my.ini with explicit --no-defaults and port override
    let ini_path = maria_dir.join("my.ini");
    let ini_content = format!(
        "[mysqld]\n\
        basedir={}\n\
        datadir={}\n\
        port={}\n\
        skip-networking=0\n\
        bind-address=127.0.0.1\n\
        character-set-server=utf8mb4\n\
        collation-server=utf8mb4_unicode_ci\n\
        innodb_buffer_pool_size=64M\n\
        skip-grant-tables\n\
        innodb_fast_shutdown=2\n",
        maria_dir.display().to_string().replace('\\', "/"),
        data_dir.display().to_string().replace('\\', "/"),
        port
    );
    std::fs::write(&ini_path, &ini_content)?;
    tracing::info!("Wrote MariaDB config to {}", ini_path.display());

    // Start MariaDB server
    tracing::info!("Starting MariaDB on port {} from {}", port, bin_dir.display());
    let mut cmd = Command::new(bin_dir.join(format!("mariadbd{}", std::env::consts::EXE_SUFFIX)));
    cmd.kill_on_drop(true)
       .arg(format!("--defaults-file={}", ini_path.display()))
       .arg("--console")
       .arg(format!("--port={}", port))
       .arg(format!("--datadir={}", data_dir.display().to_string().replace('\\', "/")))
       .current_dir(&maria_dir);
       
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    
    let child = cmd.spawn()?;
    tracing::info!("MariaDB process spawned");

    Ok(child)
}

pub async fn wait_until_ready(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    tracing::info!("Waiting for MariaDB to become ready on {}...", addr);
    let mut retries = 0;
    let max_retries = 60; // 30 seconds total (60 * 500ms)
    while retries < max_retries {
        match TcpStream::connect(&addr).await {
            Ok(_) => {
                tracing::info!("MariaDB is ready on port {} (after {} attempts)", port, retries + 1);
                return Ok(());
            }
            Err(e) => {
                if retries % 10 == 0 {
                    tracing::debug!("MariaDB not ready yet (attempt {}/{}): {}", retries + 1, max_retries, e);
                }
            }
        }
        sleep(Duration::from_millis(500)).await;
        retries += 1;
    }
    Err(AppError::Runtime(format!(
        "MariaDB failed to start within {} seconds on port {}",
        max_retries / 2, port
    )))
}
