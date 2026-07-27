use std::sync::Arc;
use tauri::{AppHandle, State, Emitter};
use serde::Serialize;
use crate::config::AppConfig;
use crate::runtime::{self, SharedRuntimeState};
use crate::archive::{self, manifest::ArchiveItem, extractor, core_fallback};
use crate::database::{self, wpconfig};
use crate::errors::{AppError, Result};

#[derive(Serialize, Clone)]
struct SiteInfo {
    name: String,
    url: String,
    wordpress_version: Option<String>,
    php_version: Option<String>,
    port: u16,
    created: Option<String>,
}

#[derive(Serialize, Clone)]
struct AllReadyPayload {
    status: String,
    site_info: SiteInfo,
}

#[tauri::command]
pub async fn get_archives(config: State<'_, AppConfig>) -> Result<Vec<ArchiveItem>> {
    archive::list_archives(&config)
}

#[tauri::command]
pub async fn start_archive(
    id: String,
    force_reextract: Option<bool>,
    app: AppHandle,
    config: State<'_, AppConfig>,
    state: State<'_, SharedRuntimeState>,
) -> Result<()> {
    let force_reextract = force_reextract.unwrap_or(false);
    tracing::info!("Starting archive with ID: {} (force_reextract: {})", id, force_reextract);
    
    let zip_path = archive::get_archive_path(&config, &id)
        .ok_or_else(|| AppError::Archive(format!("Archive not found: {}", id)))?;

    // Extract archive
    let (extract_path, newly_extracted) = extractor::extract_archive(&zip_path, &config, &id, force_reextract).await?;
    
    // Ensure WordPress core is present (for partial backups)
    core_fallback::ensure_wp_core(&extract_path, &config).await?;
    
    // Start all services
    let state_clone = Arc::clone(&*state);
    let (db_port, _php_port, http_port) = runtime::start_all(&app, &config, state_clone, &extract_path).await?;
    
    // Ensure DB exists and import SQL if not already done
    let sql_file = extract_path.join("database.sql");
    let db_name = id.replace("-", "_").replace(".", "_"); // Sanitize db name
    
    if newly_extracted {
        database::drop_database(&config, db_port, &db_name).await?;
    }
    
    database::create_database(&config, db_port, &db_name).await?;
    
    if newly_extracted && sql_file.exists() {
        let table_prefix = wpconfig::get_table_prefix(&extract_path).unwrap_or_else(|_| "wp_".to_string());
        database::restore::import_sql(&config, db_port, &db_name, &sql_file, &table_prefix).await?;
        
        // Apply package.json config for .wpress backups (activates plugins/theme)
        let package_json = extract_path.join("package.json");
        database::restore::apply_wpress_config(&config, db_port, &db_name, &package_json, &table_prefix).await?;
    }
    
    // Perform search replace to update URLs in DB
    let new_url = format!("http://localhost:{}", http_port);
    // Ideally, we'd read the old URL from manifest.json, but for now we can infer from the WP DB or skip if not needed
    // Assuming the old URL is somehow provided or handled inside search-replace script if we omit it.
    // For this plan, wp-config.php will hardcode siteurl anyway.
    
    // Update wp-config.php
    wpconfig::generate_or_patch(&extract_path, db_port, &db_name, &new_url)?;
    
    // Emit all ready
    let site_info = SiteInfo {
        name: id.clone(),
        url: new_url,
        wordpress_version: Some("Unknown".into()),
        php_version: Some("8.3".into()), 
        port: http_port,
        created: None,
    };

    app.emit("status_update", AllReadyPayload {
        status: "all_ready".into(),
        site_info,
    }).map_err(AppError::Tauri)?;

    Ok(())
}

#[tauri::command]
pub async fn stop_all(state: State<'_, SharedRuntimeState>) -> Result<()> {
    tracing::info!("Stopping all services");
    let state_clone = Arc::clone(&*state);
    runtime::stop_all(state_clone).await;
    Ok(())
}
