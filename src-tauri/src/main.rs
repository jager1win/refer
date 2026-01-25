// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
/*
use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::rolling;

fn init_tracing() -> Result<(), Box<dyn std::error::Error>>  {
    let log_dir = refer_lib::get_log_path(tauri::AppHandle);
    // папка для логов: в Tauri обычно используют tauri::api::path::app_log_dir
    let log_dir = tauri::path::BaseDirectory::AppLog.app_log_dir(&tauri::Config::default())
        .unwrap_or_else(|| std::path::PathBuf::from("./logs"));
    std::fs::create_dir_all(&log_dir).ok();

    // ротация: daily / hourly / never
    let log_appender = rolling::daily(log_dir, "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(log_appender);

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(non_blocking)  
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}*/

fn main() {
    refer_lib::run()
}
