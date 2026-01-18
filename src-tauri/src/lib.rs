use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Wry;
use tauri::{Builder, Manager};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsStore {
    pub theme: String,
    pub language: String,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            language: "en".into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct AppStatistics {
    pub settings_path: PathBuf,
    pub db_path: PathBuf,
    pub db_path_size: u32,
    pub db_count: u32,
    pub db_names: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppDb {
    pub db_path: PathBuf,
}

// universal path to config
fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // Android
        let mut path = app.path().app_local_data_dir().unwrap();
        path.push("settings.json");
        path
    }
    #[cfg(not(target_os = "android"))]
    {
        // Linux/Windows
        let mut path: PathBuf = app.path().app_config_dir().unwrap();
        path.push("settings.json");
        path
    }
}

fn load_settings(app: &tauri::AppHandle) -> SettingsStore {
    let path = settings_path(app);
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str::<SettingsStore>(&s).unwrap_or_default(),
        Err(_) => {
            // файл не найден — дефолт
            SettingsStore::default()
        }
    }
}

fn save_settings(app: &tauri::AppHandle, state: &SettingsStore) {
    if let Ok(s) = serde_json::to_string_pretty(state) {
        /*if let Some(dir) = app.path().app_dir() {
            // создаём папку, если нужно
            let _ = std::fs::create_dir_all(&dir);
        }*/
        let _ = fs::write(settings_path(app), s); // игнорируем ошибку записи (можно логировать)
    }
}

/*#[tauri::command]
fn get_settings(state: tauri::State<'_, Mutex<SettingsState>>) -> SettingsState {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(
    new: SettingsState,
    state: tauri::State<'_, Mutex<SettingsState>>,
    app: tauri::AppHandle,
) {
    {
        let mut guard = state.lock().unwrap();
        *guard = new.clone();
    }
    // асинхронно сохранить — тут просто синхронно (недолго для двух полей)
    save_settings(&app, &new);
}*/

#[tauri::command]
async fn get_settings(app: tauri::AppHandle) -> Result<SettingsStore, String> {
    let store = app.store(".settings.json").map_err(|e| e.to_string())?;

    println!("{:?}",store.get("theme"));

    // Читаем настройки или возвращаем дефолтные
    let theme = match store.get("theme") {
        Some(value) => value.as_str().unwrap_or("light").to_string(),
        None => {store.set("theme", "light");"light".into()},
    };
//
    let language = match store.get("language") {
        Some(value) => value.as_str().unwrap_or("en").to_string(),
        None => {store.set("language", "en");"en".into()},
    };

    match store.save() {
        Ok(_) => {
            println!("Settings saved successfully");
        },
        Err(e) => {
            println!("Failed to save: {}", e);
        }
    }

    Ok(SettingsStore { theme, language })
}

#[tauri::command]
async fn set_settings(app: tauri::AppHandle, new: SettingsStore) -> Result<(), String> {
    let store = app.store(".settings.json").map_err(|e| e.to_string())?;

    store.set("theme", new.theme);
    store.set("language", new.language);
    store.save().map_err(|e| e.to_string())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
/*#[tauri::command]
async fn get_settings(app: tauri::AppHandle) -> Result<AppState, String> {
    let app_dir = get_settings_path(&app);

    let settings_path = app_dir.join("settings.json");

    let mut default_settings = serde_json::json!({
        "theme": "light",
        "language": "en",
    });

    if !settings_path.exists() {
        return create_default_settings(&app_dir, &settings_path, default_settings);
    }

    match std::fs::read_to_string(&settings_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(settings) => {
                    Ok(settings)
                }
                Err(_e) => {
                    create_default_settings(&app_dir, &settings_path, default_settings)
                }
            }
        }
        Err(_e) => {
            create_default_settings(&app_dir, &settings_path, default_settings)
        }
    }
}

#[tauri::command]
async fn save_settings(app: tauri::AppHandle, settings: Value) -> Result<(), String> {
    let path = get_settings_path(&app);

    // Создать директорию если нужно
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(path, content).map_err(|e| format!("Failed to write settings: {}", e))
}

#[tauri::command]
async fn save_settings(settings: serde_json::Value, app: tauri::AppHandle) -> Result<(), String> {
    let settings_path = get_settings_path(&app);

    // Сохраняем настройки
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).unwrap()
    ).map_err(|e| format!("Не удалось сохранить настройки: {}", e))?;

    println!("Настройки сохранены в: {:?}", settings_path);
    Ok(())
}*/

//#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .setup(|app| {
            // читаем настройки (или берем дефолт)
            ////let initial = load_settings(app.handle());
            // сохраняем управляемое состояние: Mutex<AppState>
            ////app.manage(Mutex::new(initial));

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![get_settings, set_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
/*
fn create_default_settings(
    app_dir: &std::path::Path,
    settings_path: &std::path::Path,
    default_settings: serde_json::Value
) -> Result<serde_json::Value, String> {
    std::fs::create_dir_all(app_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    std::fs::write(
        settings_path,
        serde_json::to_string_pretty(&default_settings).unwrap()
    ).map_err(|e| format!("Failed to create settings.json: {}", e))?;

    Ok(default_settings)
}*/
