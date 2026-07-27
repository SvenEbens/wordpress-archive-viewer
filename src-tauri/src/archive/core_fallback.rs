use std::path::Path;
use std::fs;
use reqwest;
use std::io::{self, Cursor, Read, Write};
use zip::ZipArchive;

use crate::config::AppConfig;
use crate::errors::{AppError, Result};

use serde_json::Value;

pub async fn ensure_wp_core(extract_path: &Path, config: &AppConfig) -> Result<()> {
    let wp_config_sample = extract_path.join("wp-config-sample.php");
    let wp_admin = extract_path.join("wp-admin");

    if wp_config_sample.exists() || wp_admin.exists() {
        // Core is already present, no fallback needed
        return Ok(());
    }

    tracing::info!("Partial backup detected! Downloading and injecting fresh WordPress core...");

    // Determine which version of WP to download
    let mut wp_version = "latest".to_string();
    let package_json_path = extract_path.join("package.json");
    
    if package_json_path.exists() {
        if let Ok(content) = fs::read_to_string(&package_json_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                if let Some(v) = json.get("WordPress").and_then(|wp| wp.get("Version")).and_then(|v| v.as_str()) {
                    wp_version = v.to_string();
                    tracing::info!("Found WordPress version {} in package.json", wp_version);
                }
            }
        }
    }

    let download_url = if wp_version == "latest" {
        "https://wordpress.org/latest.zip".to_string()
    } else {
        format!("https://wordpress.org/wordpress-{}.zip", wp_version)
    };

    let cached_wp = config.cache_dir.join(format!("wp-{}.zip", wp_version));
    
    if !cached_wp.exists() {
        tracing::info!("Downloading {} to cache...", download_url);
        download_wp_core(&download_url, &cached_wp).await?;
    } else {
        tracing::info!("Using cached WordPress core ({}).", wp_version);
    }

    // 2. Extract into a temporary directory
    let temp_core_extract = config.temp_dir.join("wp-core-temp");
    if temp_core_extract.exists() {
        fs::remove_dir_all(&temp_core_extract)?;
    }
    fs::create_dir_all(&temp_core_extract)?;

    extract_wp_core(&cached_wp, &temp_core_extract)?;

    // 3. Merge the extracted `wordpress/` folder into `extract_path` without overwriting existing files
    let wordpress_dir = temp_core_extract.join("wordpress");
    if !wordpress_dir.exists() {
        return Err(AppError::Archive("Downloaded WordPress core is invalid.".into()));
    }

    merge_directories(&wordpress_dir, extract_path, true)?;

    // Cleanup temp core extract
    let _ = fs::remove_dir_all(temp_core_extract);

    Ok(())
}

async fn download_wp_core(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| AppError::Runtime(format!("Failed to download WP core: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Runtime("Failed to download WP core: Server returned error.".into()));
    }

    let bytes = response.bytes().await
        .map_err(|e| AppError::Runtime(format!("Failed to read WP core bytes: {}", e)))?;

    let mut out = fs::File::create(dest)?;
    io::copy(&mut Cursor::new(bytes), &mut out)?;

    Ok(())
}

fn extract_wp_core(archive_path: &Path, extract_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

fn merge_directories(src: &Path, dst: &Path, is_root: bool) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            merge_directories(&src_path, &dst_path, false)?;
        } else {
            if is_root || !dst_path.exists() {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    }
    Ok(())
}
