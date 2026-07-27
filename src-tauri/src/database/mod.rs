use std::path::Path;
use tokio::process::Command;

use crate::config::AppConfig;
use crate::errors::{AppError, Result};

pub mod restore;
pub mod wpconfig;

pub async fn create_database(config: &AppConfig, db_port: u16, db_name: &str) -> Result<()> {
    let maria_dir = config.runtime_dir.join("mariadb");
    let bin_dir = maria_dir.join("bin");

    tracing::info!("Creating database: {}", db_name);
    
    // Using mariadb client to create database
    let status = Command::new(bin_dir.join(format!("mariadb{}", std::env::consts::EXE_SUFFIX)))
        .arg("-h").arg("127.0.0.1")
        .arg("-P").arg(db_port.to_string())
        .arg("-u").arg("root")
        .arg("-e").arg(format!("CREATE DATABASE IF NOT EXISTS `{}`;", db_name))
        .status()
        .await?;

    if !status.success() {
        return Err(AppError::Database(format!("Failed to create database {}", db_name)));
    }

    Ok(())
}

pub async fn drop_database(config: &AppConfig, db_port: u16, db_name: &str) -> Result<()> {
    let maria_dir = config.runtime_dir.join("mariadb");
    let bin_dir = maria_dir.join("bin");

    let status = Command::new(bin_dir.join(format!("mariadb{}", std::env::consts::EXE_SUFFIX)))
        .arg("-h").arg("127.0.0.1")
        .arg("-P").arg(db_port.to_string())
        .arg("-u").arg("root")
        .arg("-e").arg(format!("DROP DATABASE IF EXISTS `{}`;", db_name))
        .status()
        .await?;

    if !status.success() {
        return Err(AppError::Database(format!("Failed to drop database {}", db_name)));
    }

    Ok(())
}
