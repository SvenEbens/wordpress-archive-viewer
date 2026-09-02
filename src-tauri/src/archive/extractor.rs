use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Read, Write};
use zip::ZipArchive;
use sha2::{Sha256, Digest};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::{AppError, Result};

pub async fn extract_archive(archive_path: &Path, config: &AppConfig, archive_id: &str, force_reextract: bool) -> Result<(PathBuf, bool)> {
    // 1. Calculate hash of the zip to check cache
    tracing::info!("Checking cache for {} (force_reextract: {})", archive_id, force_reextract);
    let hash = calculate_hash(archive_path)?;
    
    let extract_dir = config.temp_dir.join(archive_id);
    let hash_file = extract_dir.join(".archive_hash");

    // 2. Check if already extracted and valid
    if !force_reextract && extract_dir.exists() && hash_file.exists() {
        if let Ok(existing_hash) = std::fs::read_to_string(&hash_file) {
            if existing_hash == hash {
                tracing::info!("Using cached extraction for {}", archive_id);
                return Ok((extract_dir, false));
            }
        }
    }

    // 3. Clear existing dir
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }
    std::fs::create_dir_all(&extract_dir)?;

    // 4. Extract
    tracing::info!("Extracting {} to {}", archive_id, extract_dir.display());
    
    // Perform extraction in a blocking task since it's CPU/IO bound
    let extract_dir_clone = extract_dir.clone();
    let archive_path_clone = archive_path.to_path_buf();
    
    tokio::task::spawn_blocking(move || -> Result<()> {
        let ext = archive_path_clone.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        if ext == "wpress" {
            let mut reader = wpress_oxide::Reader::new(&archive_path_clone)
                .map_err(|e| AppError::Archive(format!("Failed to read wpress: {}", e)))?;
            reader.extract_to(&extract_dir_clone)
                .map_err(|e| AppError::Archive(format!("Failed to extract wpress: {}", e)))?;
        } else {
            let file = File::open(&archive_path_clone)?;
            let mut archive = ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => extract_dir_clone.join(path),
                    None => continue,
                };

                if file.name().ends_with('/') {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(&p)?;
                        }
                    }
                    let mut outfile = File::create(&outpath)?;
                    io::copy(&mut file, &mut outfile)?;
                }
            }
        }

        // Post-extraction: Check for WPvivid multi-part archives
        let wpvivid_info = extract_dir_clone.join("wpvivid_package_info.json");
        if wpvivid_info.exists() {
            tracing::info!("WPvivid archive detected, extracting internal parts...");
            
            // Read the directory to find inner zips
            let mut inner_zips = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&extract_dir_clone) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext == "zip" && path.file_name().unwrap_or_default().to_string_lossy().contains("_backup_") {
                                inner_zips.push(path);
                            }
                        }
                    }
                }
            }

            for inner_zip in inner_zips {
                tracing::info!("Extracting inner WPvivid zip: {}", inner_zip.display());
                let file = File::open(&inner_zip)?;
                let mut archive = ZipArchive::new(file)?;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let outpath = match file.enclosed_name() {
                        Some(path) => extract_dir_clone.join(path),
                        None => continue,
                    };

                    if file.name().ends_with('/') {
                        std::fs::create_dir_all(&outpath)?;
                    } else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                std::fs::create_dir_all(&p)?;
                            }
                        }
                        let mut outfile = File::create(&outpath)?;
                        io::copy(&mut file, &mut outfile)?;
                    }
                }
                
                // Optional: Delete the inner zip after extracting to save space
                let _ = std::fs::remove_file(&inner_zip);
            }
            
            // For WPvivid, the DB is often extracted as a file ending in .sql inside `wp-content/db` or the root.
            // Let's ensure it's named `database.sql` in the root so `commands.rs` finds it.
            let mut found_sql = false;
            let possible_sql_paths = vec![
                extract_dir_clone.join("wp-content/wpvividbackups/"),
                extract_dir_clone.join("wp-content/db/"),
                extract_dir_clone.clone()
            ];
            
            for dir in possible_sql_paths {
                if dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.extension().and_then(|e| e.to_str()) == Some("sql") {
                                if path != extract_dir_clone.join("database.sql") {
                                    tracing::info!("Found WPvivid SQL file: {}, moving to database.sql", path.display());
                                    let _ = std::fs::copy(&path, extract_dir_clone.join("database.sql"));
                                }
                                found_sql = true;
                                break;
                            }
                        }
                    }
                }
                if found_sql { break; }
            }
        }
        
        // Post-extraction (applies to both WPvivid and .wpress): 
        // Some backup formats put wp-content folders at the root. Move them to wp-content/
        let wp_content = extract_dir_clone.join("wp-content");
        let _ = std::fs::create_dir_all(&wp_content);
        for folder in ["plugins", "themes", "uploads", "mu-plugins", "gallery", "languages", "upgrade", "astra-local-fonts", "cache"].iter() {
            let src = extract_dir_clone.join(folder);
            let dst = wp_content.join(folder);
            if src.exists() {
                if !dst.exists() {
                    let _ = std::fs::rename(&src, &dst);
                } else {
                    // Merge contents
                    if let Ok(entries) = std::fs::read_dir(&src) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let _ = std::fs::rename(entry.path(), dst.join(entry.file_name()));
                        }
                    }
                    let _ = std::fs::remove_dir_all(&src);
                }
            }
        }
        
        Ok(())
    }).await.unwrap()?;

    // 5. Save hash
    std::fs::write(hash_file, hash)?;

    Ok((extract_dir, true))
}

fn calculate_hash(path: &Path) -> Result<String> {
    // A quick hash based on file metadata (size + modified time) is faster than a full SHA256 of the file.
    // For local archives, this is usually sufficient to detect changes.
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();
    let modified = metadata.modified()
        .unwrap_or_else(|_| std::time::SystemTime::now())
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut hasher = Sha256::new();
    hasher.update(size.to_string());
    hasher.update(modified.to_string());
    
    Ok(format!("{:x}", hasher.finalize()))
}
