use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager};
pub mod sql;
pub mod commands;
use crate::sql::*;
use crate::commands::*;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::rolling;
use tracing::{debug, error, warn, info};

const APP_EXTENSION: &str = "refer";

/*
    info!("Логирование инициализировано. Путь: {:?}", log_dir);
    warn!("Логирование инициализировано. Путь: {:?}", log_dir);

*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsStore {
    pub theme: String,
    pub language: String,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self {
            theme: "light".to_string(),
            language: "en".to_string(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsState {
    pub db_path: PathBuf,
    pub db_path_size: u64,
    pub db_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbInfo {
    pub db_size: u32,
}


//#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            init_tracing(app.handle())?;
            app.manage(Mutex::new(StatisticsState::default()));
            set_stat_all(app.handle().clone());

            Ok(())
        })
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            get_stat
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// set stat to State
fn set_stat_all(app: tauri::AppHandle) {
    let state = app.state::<Mutex<StatisticsState>>();

    // Lock the mutex to get mutable access:
    let mut state = state.lock().unwrap();

    // Modify the state:
    let doc_path: PathBuf = [app.path().document_dir().unwrap(), "refer".into()]
        .iter()
        .collect();
    state.db_path = doc_path.clone();

    let path_info = get_db_path_info(doc_path);
    state.db_path_size = path_info.0;
    state.db_list = path_info.1;

    println!("set_stat_all {:?}", state);
}

/* return StatisticsState (db_path_size,db_list) */
fn get_db_path_info(p: PathBuf) -> (u64, Vec<String>) {
    let mut total_size: u64 = 0;
    let mut names: Vec<String> = Vec::new();

    if !p.is_dir() {
        return (0, Vec::new());
    }

    // Рекурсивный обход с помощью стека (избегаем рекурсии глубиной > 1000)
    let mut stack = vec![p];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }

                if let Ok(meta) = entry.metadata() {
                    total_size += meta.len();
                }

                if let Some(ext_os) = path.extension()
                    && let Some(ext) = ext_os.to_str()
                        && ext.eq_ignore_ascii_case(APP_EXTENSION)
                            && let Some(name_os) = path.file_name() {
                                names.push(name_os.to_string_lossy().into_owned());
                            }
            }
        }
    }

    (total_size, names)
}

pub fn get_log_path(app: tauri::AppHandle)->PathBuf{
    app.path().app_log_dir().map_err(|e| e.to_string()).unwrap()
}

pub fn get_temp_path(app: tauri::AppHandle)->PathBuf{
    app.path().app_cache_dir().map_err(|e| e.to_string()).unwrap()
}

fn init_tracing(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Получаем путь к директории логов, специфичной для приложения и ОС
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&log_dir).ok();

    // 2. Настраиваем запись в файл с ротацией
    let log_appender = rolling::daily(&log_dir, "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_appender);

    // 3. Создаем и устанавливаем подписчика для tracing
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new("info")) // Уровень из RUST_LOG
        .with_writer(non_blocking)
        .with_ansi(false) // Отключаем ANSI-коды для файла
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}