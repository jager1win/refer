use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path,PathBuf};
use std::sync::Mutex;
use tauri::{Manager};
pub mod sql;
pub mod commands;
use crate::sql::*;
use crate::commands::*;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::rolling;
use tracing::{debug, error, warn, info};

const APP_EXT: &str = "refer";

#[derive(serde::Serialize)]
pub struct RError(pub String);

impl RError {
    pub fn new(code: &str) -> Self {
        Self(code.to_string())
    }
}

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
    pub log_path: PathBuf,
    pub errors: Vec<String>
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
            set_stat_all(app.handle());

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
fn set_stat_all(app: &tauri::AppHandle) {
    let state = app.state::<Mutex<StatisticsState>>();
    // Lock the mutex to get mutable access:
    let mut state = state.lock().unwrap();

    // set doc path & info
    match app.path().document_dir() {
        Ok(mut path) => {
            path.push("refer");
            state.db_path = path;

            let t = get_db_path_info(&state.db_path);
            state.db_path_size = t.0;
            state.db_list = t.1;
        }
        Err(_e) => {
            state.errors.push("DOCUMENT_DIR_MISSING".to_string());
        }
    }
    // set logs path
    match app.path().app_log_dir() {
        Ok(path) => state.log_path = path,
        Err(_e) => {
            state.errors.push("LOG_DIR_MISSING".to_string());
        }
    }

    println!("{:?}",state);
}

/* return StatisticsState (db_path_size,db_list) */
fn get_db_path_info(p: &Path) -> (u64, Vec<String>) {
    let mut total_size: u64 = 0;
    let mut names: Vec<String> = Vec::new();

    if !p.is_dir() {
        return (0, Vec::new());
    }

    // Получаем каноничный путь для корректного вычисления относительных путей
    let base_path = match p.canonicalize() {
        Ok(path) => path,
        Err(_) => p.to_path_buf(),
    };

    // Рекурсивный обход с помощью стека
    let mut stack = vec![p.to_path_buf()];
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

                // Проверяем расширение файла
                if let Some(ext_os) = path.extension()
                    && let Some(ext) = ext_os.to_str()
                    && ext.eq_ignore_ascii_case(APP_EXT)
                {
                    // Получаем относительный путь от базовой директории
                    if let Ok(relative_path) = path.strip_prefix(&base_path) {
                        // Преобразуем в строку с разделителями в стиле текущей ОС
                        let path_str = relative_path.to_string_lossy().into_owned();
                        names.push(path_str);
                    } else {
                        // Если не удалось получить относительный путь, используем имя файла
                        if let Some(name_os) = path.file_name() {
                            names.push(name_os.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
    }

    (total_size, names)
}

// enable tracing::subscriber
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
