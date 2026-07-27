use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveManifest {
    pub name: String,
    pub url: String,
    pub wordpress_version: String,
    pub php_version: String,
    pub created: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveItem {
    pub id: String, // basename without extension
    pub path: String,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub wordpress_version: Option<String>,
    pub php_version: Option<String>,
    pub created: Option<String>,
    pub thumbnail: Option<String>, // boolean flag essentially, true if exists
}
