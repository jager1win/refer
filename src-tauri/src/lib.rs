use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Builder, Manager};

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
    pub db_count: u32,
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

    println!("{:?},{:?}", config_dir, settings_path);

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
async fn get_stat(app: tauri::AppHandle) -> Result<(String, u64, u32, Vec<String>), String> {
    let doc_path: PathBuf = [app.path().document_dir().unwrap(), "refer".into()]
        .iter()
        .collect();
    let t = get_db_path_info(doc_path.clone());
    let p = doc_path.display().to_string();

    Ok((p, t.0, t.1, t.2))
}

/*#[tauri::command]
async fn get_stat(app: tauri::AppHandle,state: State<'_, Mutex<StatisticsState>>) -> Result<StatisticsState, String>{
    //let state = app.state::<Mutex<StatisticsState>>();
    let state = state.lock();
    let result:StatisticsState = set_stat1(app);
    Ok(result)
}*/

//#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(StatisticsState::default()));
            set_stat_all(app.handle().clone());

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            get_stat
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

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
    state.db_count = path_info.1;
    state.db_list = path_info.2;

    println!("set_stat_all {:?}", state);
}

/* return StatisticsState (db_path_size,db_count,db_list) */
fn get_db_path_info(p: PathBuf) -> (u64, u32, Vec<String>) {
    let mut total_size: u64 = 0;
    let mut count: u32 = 0;
    let mut names: Vec<String> = Vec::new();

    if !p.is_dir() {
        return (0, 0, Vec::new());
    }

    let db_extensions = [".sqlite", ".sqlite3", ".db"];

    // Рекурсивный обход с помощью стека (избегаем рекурсии глубиной > 1000)
    let mut stack = vec![p];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    // Добавляем размер любого файла к общему объёму
                    if let Ok(meta) = fs::metadata(&path) {
                        total_size += meta.len();
                    }

                    // Проверяем расширение только для БД
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if db_extensions.contains(&ext_lower.as_str()) {
                            count += 1;
                            if let Some(name_os) = path.file_name() {
                                // Безопасно конвертируем в String
                                names.push(name_os.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }
    }

    (total_size, count, names)
}

/*fn get_stat_one(db:String){

}*/
