use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AppConfig {
    pub archives_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppConfig {
    pub fn init(_app: &AppHandle) -> Self {
        // Normally you'd want these in AppData, but for a truly portable app
        // we'll place them relative to the executable (or current working dir).
        let mut base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        
        // For release builds, prefer the directory containing the executable.
        // This ensures shortcuts or running from terminal with a different CWD still finds the runtime next to the .exe
        #[cfg(not(debug_assertions))]
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                base_dir = parent.to_path_buf();
                
                // On macOS, if running inside a .app bundle (e.g. MyApp.app/Contents/MacOS/exe),
                // we want the base directory to be the folder containing MyApp.app.
                #[cfg(target_os = "macos")]
                {
                    if parent.ends_with("Contents/MacOS") {
                        if let Some(app_bundle) = parent.parent().and_then(|p| p.parent()) {
                            if let Some(app_parent) = app_bundle.parent() {
                                base_dir = app_parent.to_path_buf();
                            }
                        }
                    }
                }
            }
        }
        
        let config = AppConfig {
            archives_dir: base_dir.join("archives"),
            runtime_dir: base_dir.join("runtime"),
            temp_dir: base_dir.join("temp"),
            cache_dir: base_dir.join("cache"),
            logs_dir: base_dir.join("logs"),
        };

        // Ensure directories exist
        let _ = std::fs::create_dir_all(&config.archives_dir);
        let _ = std::fs::create_dir_all(&config.temp_dir);
        let _ = std::fs::create_dir_all(&config.cache_dir);
        let _ = std::fs::create_dir_all(&config.logs_dir);

        config
    }
}
