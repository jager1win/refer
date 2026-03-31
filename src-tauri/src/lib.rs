use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};
pub mod commands;
pub mod import;
pub mod sql;
use crate::commands::*;
use crate::sql::DbState;
use tracing::{error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, fmt, prelude::*};

pub const APP_EXT: &str = "refer";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsStore {
    pub theme: String,
    pub language: String,
    pub color: String,
    log: String,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self {
            theme: "light".to_string(),
            language: "en".to_string(),
            color: "blue".to_string(),
            log: "false".to_string(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsState {
    pub db_path: PathBuf,      // путь где хранятся базы
    pub db_path_size: u64,     // размер всех баз
    pub db_list: Vec<PathBuf>, // список имен баз включая пути от папки refer
    pub log_path: PathBuf,     // файл логов куда пишет tracing
    pub db_path_ok: String,    // Пустое если еще не проверяли, "Ok" если всё хорошо, иначе сообщение об ошибке
    pub initialized: bool,     // Флаг, что инициализация уже выполнена
}

// Храним WorkerGuard, чтобы логи не терялись при выходе
static LOG_GUARD: std::sync::OnceLock<WorkerGuard> = std::sync::OnceLock::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            init_tracing(app.handle())?;
            app.manage(Mutex::new(StatisticsState::default()));
            app.manage(Mutex::new(DbState::default()));
            // Только инициализация при запуске, без вызова set_stat_all
            init_stat_all(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_settings,
            set_settings,
            get_stat,
            del_ref,
            get_log,
            create_from_file,
            create_empty,
            create_example,
            get_meta,
            search_items,
            update_meta_field,
            get_el
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Инициализация состояния при запуске (вызывается один раз)
fn init_stat_all(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<Mutex<StatisticsState>>();
    let mut state = state.lock().unwrap();
    // Если уже инициализировано, выходим
    if state.initialized {
        return Ok(());
    }

    // Получаем путь к директории документов
    match app.path().document_dir() {
        Ok(mut path) => {
            path.push("refer");
            state.db_path = path.clone();

            // Проверяем доступность директории
            let check_result = check_writable_dir(&path);
            state.db_path_ok = check_result;

            // Логируем результат проверки
            if state.db_path_ok == "Ok" {
                info!("Directory /refer check: OK - {}", path.display());
            } else {
                warn!("Directory /refer check: {} - {}", state.db_path_ok, path.display());
            }

            // Получаем информацию о файлах
            let t = get_db_path_info(&state.db_path);
            state.db_path_size = t.0;
            state.db_list = t.1;
        }
        Err(e) => {
            error!("Failed to get documents directory: {}", e);
            state.db_path_ok = format!("Failed to get documents directory: {}", e);
            state.db_path_size = 0;
            state.db_list = Vec::new();
        }
    }

    // Устанавливаем путь для логов
    match app.path().app_log_dir() {
        Ok(mut path) => {
            path.push("app.log");
            state.log_path = path;
        }
        Err(e) => {
            error!("Failed to get log directory: {}", e);
        }
    }

    state.initialized = true;

    Ok(())
}

// Обновление статистики (вызывается по запросу с фронтенда)
pub async fn update_stat_all(stat_state: &State<'_, Mutex<StatisticsState>>) {
    let mut state = stat_state.lock().unwrap();

    // Обновляем только информацию о файлах (список и размер)
    if !state.db_path.as_os_str().is_empty() && state.db_path.is_dir() {
        let t = get_db_path_info(&state.db_path);
        state.db_path_size = t.0;
        state.db_list = t.1;
        /*debug!(
            "Statistics updated: {} files, {} bytes",
            state.db_list.len(),
            state.db_path_size
        );*/
    }
}

/* return StatisticsState (db_path_size,db_list) */
fn get_db_path_info(p: &Path) -> (u64, Vec<PathBuf>) {
    let mut total_size: u64 = 0;
    let mut names: Vec<PathBuf> = Vec::new();

    if !p.is_dir() {
        return (0, Vec::new());
    }

    // Попробуем просто прочитать директорию
    if fs::read_dir(p).is_err() {
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
                        //let path_str = relative_path.to_string_lossy().into_owned();
                        names.push(PathBuf::from(relative_path));
                    } else {
                        // Если не удалось получить относительный путь, используем имя файла
                        if let Some(name_os) = path.file_name() {
                            names.push(PathBuf::from(&name_os));
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
    use tracing_subscriber::fmt::time::FormatTime;

    struct MyTime;

    impl FormatTime for MyTime {
        fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
            let now = chrono::Local::now();
            write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S%.3f"))
        }
    }
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&log_dir).ok();
    let log_file_path = log_dir.join("app.log");

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    let _ = LOG_GUARD.set(guard);

    // 1. Слой для КОНСОЛИ (пишет в stdout по умолчанию)
    let console_layer = fmt::layer()
        .with_timer(MyTime)
        .with_ansi(true) // В консоли цвета полезны
        .with_filter(EnvFilter::new("debug")); // Здесь DEBUG

    // 2. Слой для ФАЙЛА
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_timer(MyTime)
        .with_ansi(false) // В файле цвета не нужны
        .with_filter(EnvFilter::new("info")); // Здесь только INFO и выше

    // 3. Собираем всё в единый Registry
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("= Refer App started =");
    Ok(())
}

/// Проверяет доступность директории для записи
/// Возвращает "Ok" если всё хорошо, иначе - сообщение об ошибке
fn check_writable_dir(dir: &Path) -> String {
    // Проверяем, существует ли директория
    if !dir.exists() {
        // Пытаемся создать
        if let Err(e) = fs::create_dir_all(dir) {
            return format!("Failed to create directory: {}", e);
        }
    } else if dir.is_file() {
        return "Path exists but is a file, not a directory".to_string();
    }

    // Проверяем возможность записи
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let test_filename = format!(".refer_write_test_{}.tmp", timestamp);
    let test_file_path = dir.join(test_filename);

    // Пытаемся создать и записать тестовый файл
    let result = (|| -> io::Result<()> {
        let mut file = File::options().create_new(true).write(true).open(&test_file_path)?;

        file.write_all(b"test")?;
        file.sync_all()?;
        Ok(())
    })();

    // Удаляем тестовый файл в любом случае
    let _ = fs::remove_file(&test_file_path);

    match result {
        Ok(_) => "Ok".to_string(),
        Err(e) => format!("Directory not writable: {}", e),
    }
}
