pub mod config;
pub mod errors;
pub mod logger;
pub mod port;
pub mod runtime;
pub mod archive;
pub mod database;
pub mod commands;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use runtime::RuntimeState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config = config::AppConfig::init(app.handle());
            logger::init(&config);
            app.manage(config);
            
            let state: runtime::SharedRuntimeState = Arc::new(Mutex::new(RuntimeState::default()));
            app.manage(state);
            
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_archives,
            commands::start_archive,
            commands::stop_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
