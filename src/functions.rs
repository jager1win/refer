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

pub fn full_pb(main: PathBuf, rel: PathBuf) -> PathBuf {
    let mut p = main;
    p.push(rel);
    p.to_path_buf()
}

pub fn sort_f_keys_h(map: &HashMap<String, String>) -> Vec<&String> {
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

pub fn f2name_v(v: &Vec<String>, names: &HashMap<String, String>) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for f in v {
        if names.contains_key(f) {
            res.push(names[f].clone());
        }
    }
    res
}

// Получить отображаемое имя поля
pub fn get_display_name(field: &str, meta: &TableMeta) -> String {
    meta.field_names
        .get(field)
        .cloned()
        .unwrap_or_else(|| field.to_string())
}

// Получить заголовок элемента для списка/заголовка
pub fn get_item_title(record: &DataRecord, meta: &TableMeta) -> String {
    if !meta.search_config.is_empty() {
        meta.search_config
            .iter()
            .filter_map(|field| record.fields.get(field))
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    } else {
        format!("ref: {}", record.id)
    }
}

