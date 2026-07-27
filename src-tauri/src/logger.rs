use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::rolling;
use crate::config::AppConfig;

pub fn init(config: &AppConfig) {
    let file_appender = rolling::daily(&config.logs_dir, "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with_writer(non_blocking)
        .with_ansi(false) // Logs files shouldn't have ANSI escape codes
        .init();
        
    tracing::info!("Logger initialized.");
}
