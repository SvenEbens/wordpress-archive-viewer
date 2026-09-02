use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tokio::net::TcpStream;
use crate::config::AppConfig;
use crate::errors::{AppError, Result};

pub async fn start(config: &AppConfig, port: u16) -> Result<tokio::process::Child> {
    let php_dir = config.runtime_dir.join("php");
    let php_cgi = php_dir.join(format!("php-cgi{}", std::env::consts::EXE_SUFFIX));

    if !php_cgi.exists() {
        return Err(AppError::Runtime("PHP-CGI not found. Did you run setup-runtime.ps1?".into()));
    }

    // Kill any leftover php-cgi processes from previous sessions
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "php-cgi.exe"])
            .creation_flags(0x08000000)
            .output()
            .await;
        sleep(Duration::from_millis(300)).await;
    }

    #[cfg(unix)]
    {
        let _ = Command::new("killall")
            .arg("php-cgi")
            .output()
            .await;
        sleep(Duration::from_millis(300)).await;
    }

    let ini_path = php_dir.join("php.ini");
    let dev_ini = php_dir.join("php.ini-development");

    if !ini_path.exists() && dev_ini.exists() {
        std::fs::copy(&dev_ini, &ini_path)?;
    }

    if ini_path.exists() {
        let content = std::fs::read_to_string(&ini_path).unwrap_or_default();
        let needs_config = !content.contains("; WordPress Archive Viewer settings");
        let needs_opcache_upgrade = content.contains("; WordPress Archive Viewer settings") && !content.contains("zend_extension=opcache");
        
        if needs_config || needs_opcache_upgrade {
            if needs_opcache_upgrade {
                // Remove the old config block and re-append with OPcache
                let marker = "; WordPress Archive Viewer settings";
                if let Some(pos) = content.find(marker) {
                    // Find the start of the block (go back to find the preceding newlines)
                    let trim_pos = content[..pos].rfind('\n').map(|p| p + 1).unwrap_or(pos);
                    let new_content = content[..trim_pos].to_string();
                    std::fs::write(&ini_path, new_content)?;
                    tracing::info!("Removed old WordPress Archive Viewer config block from php.ini for upgrade");
                }
            }
            
            let config_block = "\n\n; WordPress Archive Viewer settings\n\
extension_dir = \"ext\"\n\
zend_extension=opcache\n\
extension=curl\n\
extension=fileinfo\n\
extension=gd\n\
extension=intl\n\
extension=mbstring\n\
extension=exif\n\
extension=mysqli\n\
extension=openssl\n\
extension=pdo_mysql\n\
extension=zip\n\
extension=sodium\n\
\n\
max_execution_time = 300\n\
memory_limit = 256M\n\
post_max_size = 64M\n\
upload_max_filesize = 64M\n\
max_input_vars = 3000\n\
display_errors = Off\n\
log_errors = On\n\
error_log = php_errors.log\n\
\n\
; OPcache settings for performance\n\
opcache.enable=1\n\
opcache.enable_cli=0\n\
opcache.memory_consumption=128\n\
opcache.interned_strings_buffer=8\n\
opcache.max_accelerated_files=10000\n\
opcache.revalidate_freq=0\n\
opcache.validate_timestamps=0\n";
            let mut file = std::fs::OpenOptions::new().append(true).open(&ini_path)?;
            use std::io::Write;
            file.write_all(config_block.as_bytes())?;
            tracing::info!("Appended WordPress Archive Viewer settings (with OPcache) to php.ini");
        }
    }

    let bind_addr = format!("127.0.0.1:{}", port);
    
    tracing::info!("Starting PHP-CGI on {}", bind_addr);
    let mut cmd = Command::new(&php_cgi);
    cmd.arg("-b")
       .arg(&bind_addr)
       .arg("-c")
       .arg(php_dir.join("php.ini"))
       .current_dir(&php_dir)
       .env("PHP_FCGI_MAX_REQUESTS", "0") // Keep alive forever
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
    while retries < 10 {
        if TcpStream::connect(&addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
        retries += 1;
    }
    Err(AppError::Runtime("PHP-CGI failed to start within timeout".into()))
}
