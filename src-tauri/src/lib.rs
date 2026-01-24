use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager};

const APP_EXTENSION: &str = "refer";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsStore {
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
struct StatisticsState {
    pub db_path: PathBuf,
    pub db_path_size: u64,
    pub db_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbInfo {
    pub db_size: u32,
}

#[tauri::command]
async fn get_settings(app: tauri::AppHandle) -> Result<SettingsStore, String> {
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
async fn set_settings(app: tauri::AppHandle, new: SettingsStore) -> Result<(), String> {
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
async fn get_stat(app: tauri::AppHandle) -> Result<StatisticsState, String>{
    let state = app.state::<Mutex<StatisticsState>>();
    let state = state.lock().unwrap();
    let result: StatisticsState = state.clone();
    Ok(result)
}

//#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
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

/*fn get_stat_one(db:String){

}*/

/*
Для других типов данных (если понадобится): а лучше проверить в путях

// Конфигурация/настройки
let config_dir = app.config_dir().unwrap().join("refer");

// Временные файлы/кэш  
let cache_dir = app.cache_dir().unwrap().join("refer");

// Логи
let log_dir = app.data_dir().unwrap().join("refer").join("logs");

*/
