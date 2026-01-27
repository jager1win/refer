use std::fs;
use tauri::Manager;
use serde_json::Value;
use std::sync::Mutex;

use crate::SettingsStore;
use crate::StatisticsState;

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
pub async fn get_stat(app: tauri::AppHandle) -> Result<StatisticsState, String>{
    crate::set_stat_all(&app);
    let state = app.state::<Mutex<StatisticsState>>();
    let state = state.lock().unwrap();
    let result: StatisticsState = state.clone();
    Ok(result)
}

