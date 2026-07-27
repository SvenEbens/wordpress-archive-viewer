use std::path::{Path, PathBuf};
use std::fs::File;
use std::io;
use crate::errors::{AppError, Result};

#[derive(serde::Serialize, Clone)]
pub struct ArchiveInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created: Option<String>,
    pub wordpress_version: Option<String>,
    pub thumbnail: Option<String>,
}

pub fn list_archives(archives_dir: &Path) -> Result<Vec<ArchiveInfo>> {
    let mut archives = Vec::new();

    if !archives_dir.exists() {
        return Ok(archives);
    }

    for entry in std::fs::read_dir(archives_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("zip") {
            let file_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            
            // Try to get created date
            let metadata = entry.metadata().ok();
            let created = metadata
                .and_then(|m| m.created().ok())
                .map(|_t| {
                    // Placeholder for date formatting if we don't have chrono
                    "Recent".to_string()
                });

            archives.push(ArchiveInfo {
                id: file_name.clone(), // Using filename as ID for easier extraction path
                name: file_name,
                description: None, // Could parse from a meta.json inside zip if needed
                created,
                wordpress_version: None, // Could parse from wp-includes/version.php
                thumbnail: None,
            });
        }
    }

    Ok(archives)
}

pub fn extract_archive(zip_path: &Path, extract_path: &Path) -> Result<()> {
    tracing::info!("Extracting {} to {}", zip_path.display(), extract_path.display());
    
    if extract_path.exists() {
        tracing::info!("Archive already extracted, skipping...");
        return Ok(()); // Or clear it depending on requirements.
    }

    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_path.join(path),
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

    Ok(())
}

pub fn update_wp_config(extract_path: &Path, db_port: u16) -> Result<()> {
    let wp_config_path = extract_path.join("wp-config.php");
    if !wp_config_path.exists() {
        tracing::warn!("wp-config.php not found in archive");
        return Ok(());
    }

    let content = std::fs::read_to_string(&wp_config_path)?;
    let mut new_content = String::new();
    
    for line in content.lines() {
        if line.contains("DB_HOST") && line.contains("define") {
            new_content.push_str(&format!("define( 'DB_HOST', '127.0.0.1:{}' );\n", db_port));
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    std::fs::write(&wp_config_path, new_content)?;
    Ok(())
}
