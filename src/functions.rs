use crate::app::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wasm_bindgen::prelude::*;

pub fn validate_relative_refer_path(p: &Path) -> Result<(), ()> {
    let s = p.to_string_lossy();

    // не должен начинаться или заканчиваться на '/'
    if s.starts_with('/') || s.ends_with('/') {
        return Err(());
    }
    // запрещаем ':' и обратный слеш
    if s.contains(':') || s.contains('\\') || s.contains("//") {
        return Err(());
    }

    // каждый компонент не пустой, не "..", без управляющих символов и длина 1..=255
    for comp in s.split('/') {
        if comp.is_empty() {
            return Err(());
        }
        if comp == ".." {
            return Err(());
        }
        if comp.chars().any(|c| c.is_control()) {
            return Err(());
        }
        let len = comp.chars().count();
        if len == 0 || len > 255 {
            return Err(());
        }
    }

    Ok(())
}

pub fn read_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB * 10.0 {
        // показывать в байтах до ~10 KiB как целое
        format!("{} B", bytes)
    } else if b < KB * KB {
        // KiB с 1 знаком
        format!("{:.1} KiB", b / KB)
    } else {
        // MiB с 1 знаком (и дальше можно дополнять GiB и т.д.)
        format!("{:.1} MiB", b / (KB * KB))
    }
}

pub fn upd_stat(stat: RwSignal<StatisticsState>, now: RwSignal<String>) {
    spawn_local(async move {
        match invoke("get_stat", &JsValue::NULL).await {
            Ok(js) => {
                let res = from_value::<StatisticsState>(js).unwrap_or_default();
                stat.set(res);
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("Err: {}", error_msg));
            }
        };
    });
}

pub fn remove_refer_ext(p: &Path) -> String {
    let mut s = p.display().to_string();
    if s.ends_with(".refer") {
        s.truncate(s.len() - ".refer".len());
    }
    s
}

pub fn get_file_extension(filename: &str) -> String {
    filename.rsplit('.').next().unwrap_or("").to_string()
}

pub async fn read_file_as_bytes(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let array_buffer_promise = file.array_buffer();
    let array_buffer = wasm_bindgen_futures::JsFuture::from(array_buffer_promise)
        .await
        .map_err(|e| format!("Ошибка чтения файла: {:?}", e))?;

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

pub fn full_pb(main: PathBuf, rel: PathBuf) -> PathBuf {
    let mut p = main;
    p.push(rel);
    p.to_path_buf()
}

pub fn sort_f_keys_h<T>(map: &HashMap<String, T>) -> Vec<&String> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by(|a, b| {
        let num_a = a[2..].parse::<i32>().unwrap_or(0);
        let num_b = b[2..].parse::<i32>().unwrap_or(0);
        num_a.cmp(&num_b)
    });
    keys
}

pub fn sort_f_keys_v(keys: Vec<String>) -> Vec<String> {
    let mut sorted = keys;
    sorted.sort_by(|a, b| {
        let num_a = a[2..].parse::<i32>().unwrap_or(0);
        let num_b = b[2..].parse::<i32>().unwrap_or(0);
        num_a.cmp(&num_b)
    });
    sorted
}

pub fn f2name_v(v: Vec<String>, names: HashMap<String, String>) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for f in v {
        if names.contains_key(&f) {
            res.push(names[&f].clone());
        }
    }
    res
}

// let p = dbp(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
/*fn ensure_utf8_path(p: &std::path::Path) -> Result<&str, &'static str> {
    p.to_str().ok_or("Имя файла содержит недопустимые (не UTF-8) символы.")
}*/
//let result: Result<(), String> = from_value(js).map_err(|e| format!("deserialize failed: {e}"));
// app.rs или отдельный модуль helpers.rs
/*
    /*Effect::new(move |_| {
        log::info!("stat00: {:?}", stat.get());
        log::info!("set00: {:?}", settings.get());
        log::info!("now00: {:?}", now.get());
    });*/

fn show_success(now: RwSignal<String>, i18n: &I18nContext<Locale, I18nKeys>, key: impl Into<String>) {
    now.set(t_string!(i18n, key).to_string());
    clear_after_delay(now.clone(), 3000);
}

fn show_error(now: RwSignal<String>, i18n: &I18nContext<Locale, I18nKeys>, key: &str, details: &str) {
    now.set(format!("{}: {}", t_string!(i18n, key), details));
    clear_after_delay(now.clone(), 5000);
}

fn clear_after_delay(now: RwSignal<String>, ms: u32) {
    set_timeout(
        move || now.set("".to_string()),
        std::time::Duration::from_millis(ms as u64)
    );
}*/

// convert code 2 lang
/*
fn key_convert(r: &str) -> String {
    let i18n = use_i18n();
    match r {
        "err_test" => t_string!(i18n, err.err_test).to_string(),
        _ => t_string!(i18n, err.err_unknown).to_string(),
    }
}

                            <span
                                data-placement="right"
                                data-tooltip=t_string!(i18n, create.ttp_path)
                            >
                                "?"
                            </span>


    "crate_title": "Create",
    "edit_title": "Edit"
    let stat: RwSignal<StatisticsState> = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let selected_ref: RwSignal<String> = use_context::<RwSignal<String>>().expect("selected not found");

        spawn_local(async move {
            let commands = commands.get_untracked();
            let args = to_value(&SaveBackArgs { commands }).unwrap();
            let js = invoke("set_commands", args).await;
            let result: Result<String, String> = from_value(js).map_err(|e| format!("deserialize failed: {e}"));
            match result {
                Ok(_) => { set_status.set("Ok( Commands saved )".to_string());}
                Err(e) => set_status.set(format!("Err( Save failed: {e} )")),
            }
            let _ = invoke("request_restart", JsValue::NULL).await;
        });


*/
// remove_refer_ext(item.clone().display().to_string())
/*spawn_local(async move {
    let args = to_value(&CreateFormBack { val: form_data }).unwrap();
    let js = invoke("create", args).await;
    let result: Result<String, String> = from_value(js).map_err(|e| format!("deserialize failed: {e}"));
    log::debug!("{:?}", &result);
    match result {
        Ok(_) => {
            log::info!("create_ex: {}", &name);
            set_now(now,format!("{}: {}", tu_string!(i18n, create.ok_create), &name));
            //selected_ref.set(Some("abook.refer".to_string()));
            //active_tab.set(1);
        }
        Err(e) => set_now(now,format!("{}: {} - {}", tu_string!(i18n, create.er_create), &name,e))
    }

// Использование
async fn delete_reference(name: String) {
    match invoke("del_ref", &tauri_args!("val" => name.clone())).await {
        Ok(_) => log!("Deleted"),
        Err(e) => log!("Error: {:?}", e),
    }
}

});*/
