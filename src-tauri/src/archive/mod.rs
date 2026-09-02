use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

use crate::config::AppConfig;
use crate::errors::{AppError, Result};
use crate::archive::manifest::{ArchiveItem, ArchiveManifest};

pub mod extractor;
pub mod manifest;
pub mod core_fallback;
pub mod css_fixup;

pub fn list_archives(config: &AppConfig) -> Result<Vec<ArchiveItem>> {
    let mut archives = Vec::new();

    if !config.archives_dir.exists() {
        return Ok(archives);
    }

    let entries = std::fs::read_dir(&config.archives_dir)?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "zip" || ext == "wpress" {
                if let Ok(item) = read_archive_info(&path) {
                    archives.push(item);
                }
            }
        }
    }

    Ok(archives)
}

fn read_archive_info(path: &Path) -> Result<ArchiveItem> {
    let is_zip = path.extension().and_then(|e| e.to_str()) == Some("zip");
    
    let id = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut item = ArchiveItem {
        id: id.clone(),
        path: path.display().to_string(),
        name: path.file_stem().unwrap().to_string_lossy().to_string(),
        description: None,
        url: None,
        wordpress_version: None,
        php_version: None,
        created: None,
        thumbnail: None,
    };

    if is_zip {
        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                // Try to read manifest.json
                if let Ok(mut manifest_file) = archive.by_name("manifest.json") {
                    let mut content = String::new();
                    if manifest_file.read_to_string(&mut content).is_ok() {
                        if let Ok(manifest) = serde_json::from_str::<ArchiveManifest>(&content) {
                            item.name = manifest.name;
                            item.description = Some(manifest.description);
                            item.url = Some(manifest.url);
                            item.wordpress_version = Some(manifest.wordpress_version);
                            item.php_version = Some(manifest.php_version);
                            item.created = Some(manifest.created);
                        }
                    }
                }

                // Check for thumbnail
                if archive.by_name("thumbnail.png").is_ok() || archive.by_name("thumbnail.jpg").is_ok() {
                    item.thumbnail = Some("thumbnail.png".to_string()); // Simplified
                }
            }
        }
    } else {
        // .wpress file has no standard manifest.json. Just use the filename.
        item.description = Some("All-in-One WP Migration Backup (.wpress)".into());
    }

    Ok(item)
}
    
    // Handled in block above


pub fn get_archive_path(config: &AppConfig, id: &str) -> Option<PathBuf> {
    let zip_path = config.archives_dir.join(format!("{}.zip", id));
    if zip_path.exists() {
        return Some(zip_path);
    }
    let wpress_path = config.archives_dir.join(format!("{}.wpress", id));
    if wpress_path.exists() {
        return Some(wpress_path);
    }
    None
}
