use crate::sql::{self, *};
use crate::{APP_EXT, DbState, SettingsStore, StatisticsState};
use crate::import::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::fs::{self};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tracing::{debug, error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateForm {
    pub mode: String, // "empty", "sheet", "sqlite"
    pub db_name: PathBuf,
    pub has_header: bool,
    pub file_extension: String,
    pub file_path: Option<PathBuf>,
}

#[tauri::command]
pub async fn get_app_info() -> Vec<String> {
    let mut result = Vec::new();
    result.push(format!("Version: {}", env!("CARGO_PKG_VERSION")));
    result.push(format!("Author: {}", env!("CARGO_PKG_AUTHORS")));
    result.push(format!("License: {}", env!("CARGO_PKG_LICENSE")));
    result.push(format!("Githab: {}", env!("CARGO_PKG_REPOSITORY")));

    result
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
pub async fn get_stat(stat_state: State<'_, Mutex<StatisticsState>>) -> Result<StatisticsState, String> {
    crate::update_stat_all(&stat_state).await;
    let state = stat_state.lock().unwrap();
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
pub async fn get_log(app: tauri::AppHandle) -> Result<String, String> {
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

#[tauri::command]
pub async fn create_empty(val: CreateForm, stat_state: State<'_, Mutex<StatisticsState>>) -> Result<(), String> {
    let s_state = stat_state.lock().unwrap();
    let mut root = s_state.db_path.clone();

    match build_and_create_refer_path(&root, &val.db_name, val.mode == "example") {
        Ok(p) => root = p,
        Err(e) => {
            error!("Failed to create path: {}", e);
            return Err(format!("Failed to create path: {:?}", e));
        }
    }

    println!(
        "val.mode: {:?}, val.db_name: {:?}, header:{:?}, file_ext: {:?}",
        &val.mode, &val.db_name, &val.has_header, &val.file_extension
    );

    match create_empty_database(&root) {
        Ok(()) => {
            info!("Empty database created: {:?}", &val.db_name);
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&root);
            error!("Failed to create empty db: {}", e);
            Err(format!("Failed to create db: {}", e))
        }
    }
}

#[tauri::command]
pub async fn create_example(val: CreateForm, stat_state: State<'_, Mutex<StatisticsState>>) -> Result<(), String> {
    let s_state = stat_state.lock().unwrap();
    let mut root = s_state.db_path.clone();

    match build_and_create_refer_path(&root, &val.db_name, val.mode == "example") {
        Ok(p) => root = p,
        Err(e) => {
            error!("Failed to create path: {}", e);
            return Err(format!("Failed to create path: {:?}", e));
        }
    }

    println!(
        "val.mode: {:?}, val.db_name: {:?}, header:{:?}, file_ext: {:?}",
        &val.mode, &val.db_name, &val.has_header, &val.file_extension
    );

    let demo_name = &val.db_name.display().to_string();
    match create_example_database(demo_name, &root) {
        Ok(()) => {
            info!("Demo database '{}' created", demo_name);
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&root);
            error!("Failed to create demo db '{}': {}", demo_name, e);
            Err(format!("Failed to create demo db: {}", e))
        }
    }
}

#[tauri::command]
pub async fn create_from_file(
    mut val: CreateForm, app: tauri::AppHandle, stat_state: State<'_, Mutex<StatisticsState>>,
) -> Result<(), String> {
    let s_state = stat_state.lock().unwrap();
    let mut root = s_state.db_path.clone();

    match build_and_create_refer_path(&root, &val.db_name, val.mode == "example") {
        Ok(p) => root = p,
        Err(e) => {
            error!("Failed to create path: {}", e);
            return Err(format!("Failed to create path: {:?}", e));
        }
    }

    // 1. Настраиваем фильтры диалога
    let mut dialog = app.dialog().file();
    dialog = match val.mode.as_str() {
        "sheet" => dialog.add_filter("Tables", &["csv", "tsv", "xls", "xlsx", "ods"]),
        "sqlite" => dialog.add_filter("SQLite DB", &["sqlite", "sqlite3", "db"]),
        _ => dialog,
    };

    let file_path = dialog.blocking_pick_file();
    let Some(path_obj) = file_path else {
        return Err("CANCELLED".into());
    };
    let path = path_obj.as_path().ok_or("Invalid Path")?;
    val.file_path = Some(path.to_path_buf());
    val.db_name = root;

    // 2. Проверка расширения (на всякий случай, если в ОС нет фильтров)
    val.file_extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    debug!(
        "val.mode: {:?}, val.db_name: {:?}, header:{:?}, val.file_extension: {:?}, path: {:?}",
        &val.mode, &val.db_name, &val.has_header, &val.file_extension, &path
    );
    /*match create_from_csv_file(&val){
        Ok(()) => {
            info!("Database '{:?}' from file created", &val.db_name);
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&val.db_name);
            error!("Failed to create db from file: {}", e);
            Err(format!("Failed to create db from file: {:?}", e))
        }
    }*/
    match val.file_extension.as_ref() {
        "csv" | "tsv" => match create_from_csv_file(&val) {
            Ok(()) => {
                info!("Database '{:?}' from csv file created", &val.db_name);
                Ok(())
            }
            Err(e) => {
                let _ = fs::remove_file(&val.db_name);
                error!("Failed to create db from csv file: {}", e);
                Err(format!("Failed to create db from csv file: {:?}", e))
            }
        },
        "xlsx" | "ods" | "xls" => match create_from_sheet_file(&val) {
            Ok(()) => {
                info!("Database '{:?}' from sheet file created", &val.db_name);
                Ok(())
            }
            Err(e) => {
                let _ = fs::remove_file(&val.db_name);
                error!("Failed to create db from sheet file: {}", e);
                Err(format!("Failed to create db from sheet file: {:?}", e))
            }
        }
        "sqlite" | "sqlite3" | "db" => match create_from_sqlite(&val) {
            Ok(()) => {
                info!("Database '{:?}' from sqlite created", &val.db_name);
                Ok(())
            }
            Err(e) => {
                let _ = fs::remove_file(&val.db_name);
                error!("Failed to create db from sqlite: {}", e);
                Err(format!("Failed to create db from sqlite: {:?}", e))
            }
        },
        _ => Err(format!("Unknown operation: {}", val.mode)),
    }
}

fn build_and_create_refer_path(root: &Path, p: &Path, example: bool) -> Result<PathBuf, io::Error> {
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

#[tauri::command]
pub async fn get_meta(pb: PathBuf, state: State<'_, Mutex<DbState>>) -> Result<TableMeta, String> {
    let mut db = state.lock().map_err(|e| e.to_string())?;
    db.with_conn(pb.clone(), |_conn, meta| {
        let Some(meta_ref) = meta else {
            error!("Error: table meta not available for {:?}", pb);
            return Err(format!("Error: table meta not available for {:?}", pb));
        };
        Ok(meta_ref.clone())
    })
}

#[tauri::command]
pub async fn search_items(
    pb: PathBuf, query: String, state: State<'_, Mutex<DbState>>,
) -> Result<Vec<DataRecord>, String> {
    let mut db = state.lock().map_err(|e| e.to_string())?;
    db.with_conn(pb.clone(), |conn, meta| {
        let Some(meta) = meta else {
            error!("Error: table meta not available for {:?}", pb);
            return Err(format!("Error: table meta not available for {:?}", pb));
        };

        match sql::search_items(conn, meta, &query).map_err(|e| e.to_string()) {
            Ok(h) => Ok(h),
            Err(e) => {
                warn!("{}", e);
                Err(e)
            }
        }
    })
}

#[tauri::command]
pub async fn save_search_config(pb: PathBuf, vec: Vec<String>, state: State<'_, Mutex<DbState>>) -> Result<(), String> {
    let mut db = state.lock().map_err(|e| e.to_string())?;

    db.update_meta(&pb, |meta| {
        meta.search_config = vec.clone();
        Ok(())
    })?;

    db.with_conn(pb, |conn, meta| match sql::save_search_config(conn, &vec) {
        Ok(()) => {
            info!(
                "Search config saved: {:?}",
                f2name(vec, meta.as_ref().unwrap().field_names.clone())
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to save search config: {}", e);
            Err(e.to_string())
        }
    })
}

#[tauri::command]
pub async fn add_field(
    pb: PathBuf, display_name: String, field_type: String, state: State<'_, Mutex<DbState>>,
) -> Result<String, String> {
    // 1. Получаем доступ к состоянию
    let mut db = state.lock().map_err(|e| e.to_string())?;

    // 2. Вычисляем next_index из текущего meta
    let next_index = db.meta.as_ref().map(|m| m.field_names.len() + 1).unwrap_or(1);

    // 3. Получаем соединение (но не через with_conn, чтобы не заимствовать db)
    let conn = match db.conn.as_ref() {
        Some(c) => c,
        None => return Err("No active connection".to_string()),
    };

    // Проверяем путь
    if db.current_path.as_ref() != Some(&pb) {
        return Err("Wrong database".to_string());
    }

    // 4. Добавляем поле в БД
    let field_name = sql::add_field(conn, next_index, Some(&display_name), &field_type).map_err(|e| e.to_string())?;

    // 5. Обновляем meta в памяти (прямая модификация state)
    if let Some(meta) = db.meta.as_mut() {
        meta.field_names.insert(field_name.clone(), display_name);
        meta.field_types.insert(
            field_name.clone(),
            match field_type.as_str() {
                "number" => FieldType::Number,
                _ => FieldType::Text,
            },
        );
    }

    Ok(field_name)
}

fn f2name(v: Vec<String>, names: HashMap<String, String>) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for f in v {
        if names.contains_key(&f) {
            res.push(names[&f].clone());
        }
    }
    res
}

fn try_remove(path: &std::path::Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()), // нет файла — нормально
        Err(e) => Err(e),
    }
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
