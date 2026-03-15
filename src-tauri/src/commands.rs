use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::fs::{self};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

use crate::sql::{self, *};
use crate::{APP_EXT, DbState, SettingsStore, StatisticsState};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateForm {
    pub mode: String, // "empty", "sheet", "sqlite"
    pub db_name: PathBuf,
    pub has_header: bool,
    pub file_extension: Option<String>,
    pub file_data: Option<Vec<u8>>,
}

#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<SettingsStore, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let settings_path = config_dir.join(".settings.json");

    if !config_dir.exists() {
        return Ok(SettingsStore::default());
    }

    //println!("{:?},{:?}", config_dir, settings_path);

    let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let theme = json
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("light")
        .to_string();

    let language = json
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en")
        .to_string();

    let color = json.get("color").and_then(|v| v.as_str()).unwrap_or("blue").to_string();
    let log = json.get("log").and_then(|v| v.as_str()).unwrap_or("false").to_string();
    //let settings: SettingsStore = serde_json::from_value(json).unwrap_or_default();

    Ok(SettingsStore {
        theme,
        language,
        color,
        log,
    })
}

#[tauri::command]
pub async fn set_settings(app: tauri::AppHandle, new: SettingsStore) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let settings_path = config_dir.join(".settings.json");

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json_data = serde_json::to_string_pretty(&new).map_err(|e| e.to_string())?;
    fs::write(&settings_path, json_data).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_stat(app: tauri::AppHandle) -> Result<StatisticsState, String> {
    crate::update_stat_all(&app);
    let state = app.state::<Mutex<StatisticsState>>();
    let state = state.lock().unwrap();
    let result: StatisticsState = state.clone();
    Ok(result)
}

#[tauri::command]
pub async fn del_ref(app: tauri::AppHandle, val: PathBuf) -> Result<String, String> {
    let document_dir = app.path().document_dir().map_err(|e| e.to_string())?;
    let reference_path = document_dir.join(APP_EXT).join(&val);

    match fs::remove_file(reference_path) {
        Ok(_i) => Ok({
            info!("reference deleted: {:?}", val);
            String::from("reference_deleted")
        }),
        Err(e) => Ok({
            error!("failed reference deleted: {}", e);
            "failed_reference_deleted".to_string()
        }),
    }
}

/// Команда для получения содержимого лог-файла
#[tauri::command]
pub fn get_log(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<Mutex<StatisticsState>>();
    let state = state.lock().unwrap();

    let log_path = &state.log_path;

    // Проверяем, существует ли файл логов
    if !log_path.exists() {
        return Err("Log file does not exist".to_string());
    }

    // Читаем содержимое файла
    let mut file = match File::open(log_path) {
        Ok(file) => file,
        Err(e) => return Err(format!("Failed to open log file: {}", e)),
    };

    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(_) => Ok(contents),
        Err(e) => Err(format!("Failed to read log file: {}", e)),
    }
}

/// Команда для очистки лог-файла (без перезапуска приложения)
#[tauri::command]
pub fn clear_log(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<Mutex<StatisticsState>>();
    let state = state.lock().unwrap();

    let log_path = &state.log_path;

    // Проверяем, существует ли файл логов
    if !log_path.exists() {
        return Err("Log file does not exist".to_string());
    }

    // Пытаемся очистить файл
    match fs::write(log_path, "") {
        Ok(_) => Ok("Log file cleared successfully".to_string()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(
            "Note: Log file may be locked by another application. It will be cleared on next app restart.".to_string(),
        ),
        Err(e) => Err(format!("Failed to clear log file: {}", e)),
    }
}

#[tauri::command]
pub fn create(val: CreateForm, stat_state: State<'_, Mutex<StatisticsState>>) -> Result<String, String> {
    let stat_state = stat_state.lock().unwrap();
    let mut root = stat_state.db_path.clone();

    match build_and_create_refer_path(&root, &val.db_name, val.mode == "example") {
        Ok(p) => root = p,
        Err(e) => {
            error!("Failed to create path: {}", e);
            return Err(format!("Failed to create path: {:?}", e));
        }
    }

    //println!("val.mode: {:?}, val.db_name: {:?}", &val.mode, &val.db_name);
    match val.mode.as_str() {
        "empty" => match create_empty_database(&root) {
            Ok(()) => {
                info!("Empty database created: {:?}", &val.db_name);
                Ok(String::from("ok"))
            }
            Err(e) => {
                error!("Failed to create empty db: {}", e);
                Err(format!("Failed to create db: {}", e))
            }
        },
        "example" => {
            let demo_name = &val.db_name.display().to_string();
            match create_example_database(demo_name, &root) {
                Ok(()) => {
                    info!("Demo database '{}' created", demo_name);
                    Ok(String::from("ok"))
                }
                Err(e) => {
                    error!("Failed to create demo db '{}': {}", demo_name, e);
                    Err(format!("Failed to create demo db: {}", e))
                }
            }
        }
        "from_file" => match create_from_file(&root, &val) {
            Ok(()) => Ok(String::from("ok")),
            Err(e) => Err(format!("Failed to create db: {:?}", e)),
        },
        _ => Err(format!("Unknown operation: {}", val.mode)),
    }
}

pub fn build_and_create_refer_path(root: &Path, p: &Path, example: bool) -> Result<PathBuf, io::Error> {
    // Reject absolute on target platform or any prefix/root components
    if p.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "incoming path is absolute"));
    }

    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "parent-dir (`..`) not allowed",
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "absolute/prefix component not allowed",
                ));
            }
            Component::Normal(os) => {
                // Доп. проверка: запретить пустые имена или недопустимые байты
                if os.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty path component"));
                }
            }
            _ => {}
        }
    }

    // Собираем полный путь и нормализуем слэши косвенно (Path на Unix/Windows)
    let full = root.join(p);

    // Создаём директории, если нужно (безопасно, только внутри root)
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if example {
        let _ = try_remove(&full);
    }

    // Попробуем атомарно создать файл, не перезаписывая существующий
    let _f = fs::OpenOptions::new()
        .write(true)
        .create_new(true) // если файл уже есть — ошибка
        .open(&full)?;

    Ok(full)
}

fn try_remove(path: &std::path::Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()), // нет файла — нормально
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn get_meta(pb: PathBuf, state: State<'_, Mutex<DbState>>) -> Result<TableMeta, String> {
    let mut db = state.lock().map_err(|e| e.to_string())?;
    db.with_conn(pb, |_conn, meta| {
        let Some(meta_ref) = meta else {
            error!("Error: table meta not available");
            return Err("Error: table meta not available".into());
        };
        Ok(meta_ref.clone())
    })
}

//search_items
#[tauri::command]
pub fn search_items(pb: PathBuf, query: String, state: State<'_, Mutex<DbState>>) -> Result<Vec<DataRecord>, String> {
    let mut db = state.lock().map_err(|e| e.to_string())?;
    db.with_conn(pb, |conn, meta| {
        let Some(meta) = meta else {
            error!("Error: table meta not available");
            return Err("Error: table meta not available".into());
        };

        match sql::search_items(conn, meta, &query).map_err(|e| e.to_string()) {
            Ok(h) => {
                //info!("search_items ok: {}", serde_json::to_string(&h).unwrap());
                Ok(h)
            }
            Err(e) => {
                error!("{}", e);
                Err(e)
            }
        }
    })
}

/*
   let mut path = root.join(String::from("example"));
   let _ = std::fs::create_dir_all(&path);
   println!("pb: {:?}", &path);
   let file_name = format!("{}.refer", val.db_name.to_string_lossy());
   path = path.join(file_name);
   println!("pb1: {:?}", &path);
   let dd = try_remove(&path);
   println!("pb2: {:?}", &dd);
   match create_example_database(val.db_name.to_string_lossy().to_string(), path){
       Ok(()) => Ok("".to_string()),
       Err(e) => Err(format!("Failed to create: {:?}", e)),
   }
*/
