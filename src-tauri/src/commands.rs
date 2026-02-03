use std::fs::{self, OpenOptions};
use std::fs::File;
use std::io::{self, Read};
use tauri::Manager;
use serde_json::Value;
use std::sync::Mutex;

use crate::{APP_EXT, LOG_GUARD};
use crate::SettingsStore;
use crate::StatisticsState;
use tracing::{error,info};

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
    //let settings: SettingsStore = serde_json::from_value(json).unwrap_or_default();

    Ok(SettingsStore { theme, language })
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
pub async fn del_ref(app: tauri::AppHandle, val: String) -> Result<String, String> {
    let document_dir = app.path().document_dir().map_err(|e| e.to_string())?;
    let reference_path = document_dir.join(APP_EXT).join(&val);

    match fs::remove_file(reference_path){
        Ok(_i) => Ok({info!("reference deleted: {}",val);String::from("reference_deleted")}),
        Err(e) => Ok({error!("failed reference deleted: {}",e);"failed_reference_deleted".to_string()})
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
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            Ok("Note: Log file may be locked by another application. It will be cleared on next app restart.".to_string())
        }
        Err(e) => Err(format!("Failed to clear log file: {}", e)),
    }
}