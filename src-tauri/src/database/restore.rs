use std::path::Path;
use tokio::process::Command;
use crate::config::AppConfig;
use crate::errors::{AppError, Result};

pub async fn import_sql(config: &AppConfig, db_port: u16, db_name: &str, sql_file: &Path, table_prefix: &str) -> Result<()> {
    let maria_dir = config.runtime_dir.join("mariadb");
    let bin_dir = maria_dir.join("bin");

    tracing::info!("Importing SQL file: {} (table_prefix: {})", sql_file.display(), table_prefix);

    let mut child = Command::new(bin_dir.join(format!("mariadb{}", std::env::consts::EXE_SUFFIX)))
        .arg("-h").arg("127.0.0.1")
        .arg("-P").arg(db_port.to_string())
        .arg("-u").arg("root")
        .arg(db_name)
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let sql_file_path = sql_file.to_path_buf();
    let prefix = table_prefix.to_string();

    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::fs::File;
        
        if let Ok(file) = File::open(sql_file_path).await {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            
            while let Ok(bytes) = reader.read_line(&mut line).await {
                if bytes == 0 {
                    break;
                }
                
                let mut out_line = line.as_str();
                let mut replaced: String;
                if out_line.contains("SERVMASK_PREFIX_") {
                    replaced = out_line.replace("SERVMASK_PREFIX_", &prefix);
                    out_line = &replaced;
                }
                
                if stdin.write_all(out_line.as_bytes()).await.is_err() {
                    break; // Broken pipe or other error
                }
                line.clear();
            }
        }
    });

    let status = child.wait().await?;

    if !status.success() {
        return Err(AppError::Database("Failed to import SQL file".into()));
    }

    Ok(())
}

pub async fn search_replace_urls(
    config: &AppConfig,
    db_port: u16,
    db_name: &str,
    old_url: &str,
    new_url: &str,
) -> Result<()> {
    let php_dir = config.runtime_dir.join("php");
    let php_exe = php_dir.join(format!("php{}", std::env::consts::EXE_SUFFIX));
    let script_path = std::env::current_dir().unwrap().join("scripts").join("search-replace.php");

    if !script_path.exists() {
        return Err(AppError::Database("search-replace.php script not found".into()));
    }

    tracing::info!("Running search & replace: {} -> {}", old_url, new_url);

    // Usage: php search-replace.php <host> <port> <user> <password> <database> <search> <replace>
    let status = Command::new(php_exe)
        .arg(&script_path)
        .arg("127.0.0.1")
        .arg(db_port.to_string())
        .arg("root")
        .arg("") // empty password
        .arg(db_name)
        .arg(old_url)
        .arg(new_url)
        .status()
        .await?;

    if !status.success() {
        return Err(AppError::Database("Search and replace script failed".into()));
    }

    Ok(())
}

pub async fn apply_wpress_config(
    config: &AppConfig,
    db_port: u16,
    db_name: &str,
    package_json_path: &Path,
    table_prefix: &str,
) -> Result<()> {
    if !package_json_path.exists() {
        return Ok(());
    }

    let php_dir = config.runtime_dir.join("php");
    let php_exe = php_dir.join(format!("php{}", std::env::consts::EXE_SUFFIX));
    let script_path = std::env::current_dir().unwrap().join("scripts").join("apply-wpress-config.php");

    if !script_path.exists() {
        tracing::warn!("apply-wpress-config.php script not found, skipping");
        return Ok(());
    }

    tracing::info!("Applying WPRESS config from package.json");

    // Usage: php apply-wpress-config.php <package_json_path> <host> <port> <user> <pass> <db> <prefix>
    let status = Command::new(php_exe)
        .arg(&script_path)
        .arg(package_json_path)
        .arg("127.0.0.1")
        .arg(db_port.to_string())
        .arg("root")
        .arg("") // empty password
        .arg(db_name)
        .arg(table_prefix)
        .status()
        .await?;

    if !status.success() {
        tracing::warn!("Failed to apply WPRESS config");
    }

    Ok(())
}

pub async fn get_site_url(config: &AppConfig, db_port: u16, db_name: &str, table_prefix: &str) -> Result<String> {
    let maria_dir = config.runtime_dir.join("mariadb");
    let bin_dir = maria_dir.join("bin");
    
    let output = Command::new(bin_dir.join(format!("mariadb{}", std::env::consts::EXE_SUFFIX)))
        .arg("-h").arg("127.0.0.1")
        .arg("-P").arg(db_port.to_string())
        .arg("-u").arg("root")
        .arg("--skip-column-names")
        .arg("-e").arg(format!("SELECT option_value FROM {}options WHERE option_name = 'siteurl'", table_prefix))
        .arg(db_name)
        .output()
        .await?;
        
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err(AppError::Database("Could not fetch siteurl".into()));
    }
    Ok(url)
}

/// Configure Elementor to use its existing CSS files rather than trying
/// to regenerate them.  Sets `elementor_css_print_method` to `internal`
/// and clears stale Elementor transients that would otherwise trigger
/// a cache rebuild.
pub async fn fix_elementor_options(
    config: &AppConfig,
    db_port: u16,
    db_name: &str,
    table_prefix: &str,
) -> Result<()> {
    let maria_dir = config.runtime_dir.join("mariadb");
    let bin_dir = maria_dir.join("bin");

    let sql = format!(
        "UPDATE {prefix}options SET option_value = 'internal' WHERE option_name = 'elementor_css_print_method'; \
         DELETE FROM {prefix}options WHERE option_name LIKE '_transient_elementor%'; \
         DELETE FROM {prefix}options WHERE option_name LIKE '_transient_timeout_elementor%'; \
         DELETE FROM {prefix}options WHERE option_name LIKE '_site_transient_elementor%'; \
         DELETE FROM {prefix}options WHERE option_name LIKE '_site_transient_timeout_elementor%';",
        prefix = table_prefix
    );

    tracing::info!("Fixing Elementor options (css_print_method → internal, clearing transients)");

    let status = Command::new(bin_dir.join(format!("mariadb{}", std::env::consts::EXE_SUFFIX)))
        .arg("-h").arg("127.0.0.1")
        .arg("-P").arg(db_port.to_string())
        .arg("-u").arg("root")
        .arg("-e").arg(&sql)
        .arg(db_name)
        .status()
        .await?;

    if !status.success() {
        tracing::warn!("Failed to fix Elementor options (non-fatal)");
    }

    Ok(())
}
