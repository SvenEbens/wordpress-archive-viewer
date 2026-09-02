use std::path::Path;
use uuid::Uuid;
use crate::errors::{AppError, Result};

pub fn generate_or_patch(
    wp_dir: &Path,
    db_port: u16,
    db_name: &str,
    new_url: &str,
) -> Result<()> {
    let config_path = wp_dir.join("wp-config.php");
    let sample_path = wp_dir.join("wp-config-sample.php");

    let mut content = String::new();

    if config_path.exists() {
        content = std::fs::read_to_string(&config_path)?;
    } else if sample_path.exists() {
        content = std::fs::read_to_string(&sample_path)?;
    } else {
        return Err(AppError::Archive("Neither wp-config.php nor wp-config-sample.php found in archive".into()));
    }

    // Replace DB variables
    content = replace_define(&content, "DB_NAME", db_name);
    content = replace_define(&content, "DB_USER", "root");
    content = replace_define(&content, "DB_PASSWORD", "");
    content = replace_define(&content, "DB_HOST", &format!("127.0.0.1:{}", db_port));

    // Force URLs via constants
    content = ensure_define(&content, "WP_SITEURL", new_url);
    content = ensure_define(&content, "WP_HOME", new_url);
    
    // Settings for archives — use boolean defines (no quotes around value)
    content = ensure_define_raw(&content, "WP_DEBUG", "false");
    content = ensure_define_raw(&content, "WP_DEBUG_DISPLAY", "false");
    content = ensure_define_raw(&content, "WP_DEBUG_LOG", "false");
    content = ensure_define_raw(&content, "DISABLE_WP_CRON", "true");
    content = ensure_define_raw(&content, "WP_CACHE", "false");
    content = ensure_define_raw(&content, "FORCE_SSL_ADMIN", "false");
    content = ensure_define_raw(&content, "WP_HTTP_BLOCK_EXTERNAL", "true");
    content = ensure_define(&content, "AUTOMATIC_UPDATER_DISABLED", "true");
    content = ensure_define(&content, "DISALLOW_FILE_EDIT", "true");
    // Note: don't set DISALLOW_FILE_MODS as it prevents theme/plugin updates from working

    // Generate fresh salts if using sample config
    if !config_path.exists() {
        content = generate_salts(content);
    }

    // Removed error suppression to allow WP_DEBUG to work correctly
    // The WordPress Archive Viewer previously suppressed all errors, which caused whitescreens on fatal errors.

    std::fs::write(config_path, content)?;
    Ok(())
}

pub fn get_table_prefix(wp_dir: &Path) -> Result<String> {
    let config_path = wp_dir.join("wp-config.php");
    if !config_path.exists() {
        return Ok("wp_".to_string()); // fallback
    }
    
    let content = std::fs::read_to_string(&config_path)?;
    let regex = regex::Regex::new(r#"\$table_prefix\s*=\s*['"]([^'"]+)['"]\s*;"#).unwrap();
    if let Some(captures) = regex.captures(&content) {
        if let Some(prefix) = captures.get(1) {
            return Ok(prefix.as_str().to_string());
        }
    }
    
    Ok("wp_".to_string())
}

fn replace_define(content: &str, key: &str, value: &str) -> String {
    let regex = regex::Regex::new(&format!(r#"define\(\s*['"]{}['"]\s*,\s*['"][^'"]*['"]\s*\);"#, key)).unwrap();
    if regex.is_match(content) {
        regex.replace(content, format!("define( '{}', '{}' );", key, value).as_str()).into_owned()
    } else {
        content.to_string()
    }
}

fn ensure_define(content: &str, key: &str, value: &str) -> String {
    let regex = regex::Regex::new(&format!(r#"define\(\s*['"]{}['"]\s*,.*?\);"#, key)).unwrap();
    if regex.is_match(content) {
        regex.replace(content, format!("define( '{}', '{}' ); // Forced by Viewer", key, value).as_str()).into_owned()
    } else {
        // Find "That's all, stop editing!" and insert before it
        let stop_editing = "/* That's all, stop editing!";
        if let Some(pos) = content.find(stop_editing) {
            let mut new_content = String::from(&content[..pos]);
            new_content.push_str(&format!("define( '{}', '{}' ); // Forced by Viewer\n", key, value));
            new_content.push_str(&content[pos..]);
            new_content
        } else {
            // Just append
            format!("{}\ndefine( '{}', '{}' );\n", content, key, value)
        }
    }
}

/// Like ensure_define but outputs raw value without quotes (for booleans like true/false)
fn ensure_define_raw(content: &str, key: &str, raw_value: &str) -> String {
    let regex = regex::Regex::new(&format!(r#"define\(\s*['"]{}['"]\s*,.*?\);"#, key)).unwrap();
    if regex.is_match(content) {
        regex.replace(content, format!("define( '{}', {} ); // Forced by Viewer", key, raw_value).as_str()).into_owned()
    } else {
        let stop_editing = "/* That's all, stop editing!";
        if let Some(pos) = content.find(stop_editing) {
            let mut new_content = String::from(&content[..pos]);
            new_content.push_str(&format!("define( '{}', {} ); // Forced by Viewer\n", key, raw_value));
            new_content.push_str(&content[pos..]);
            new_content
        } else {
            format!("{}\ndefine( '{}', {} );\n", content, key, raw_value)
        }
    }
}

fn generate_salts(mut content: String) -> String {
    let keys = [
        "AUTH_KEY", "SECURE_AUTH_KEY", "LOGGED_IN_KEY", "NONCE_KEY",
        "AUTH_SALT", "SECURE_AUTH_SALT", "LOGGED_IN_SALT", "NONCE_SALT"
    ];

    for key in keys {
        let salt = Uuid::new_v4().to_string(); // Simple random string, better salts could be used
        content = replace_define(&content, key, &salt);
    }
    content
}
