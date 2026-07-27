use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP Error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tauri Error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("Port Error: No free ports available")]
    NoFreePort,

    #[error("Runtime Error: {0}")]
    Runtime(String),

    #[error("Database Error: {0}")]
    Database(String),

    #[error("Archive Error: {0}")]
    Archive(String),
}

// Implement standard mapping to string for Tauri IPC
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
