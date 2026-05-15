use crate::{app::*, tauri_args};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub struct State {
    pub stat: RwSignal<StatisticsState>,
    pub selected: RwSignal<Selected>,
    pub meta: RwSignal<Option<TableMeta>>,
    pub now: RwSignal<String>,
    pub edit_ref: RwSignal<bool>,
    pub settings: RwSignal<AppSettings>,
}

impl State {
    pub fn load_meta(&self) {
        let s = *self;
        spawn_local(async move {
            let pb = s.get_full_pb(s.selected.get_untracked().refer.unwrap());
            match invoke("get_meta", &tauri_args!("pb": pb)).await {
                Ok(js) => {
                    let res = from_value::<TableMeta>(js).unwrap();
                    s.meta.set(Some(res))
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    s.now.set(format!("Error: {}", error_msg));
                }
            };
        });
    }
    // ------- Static functions -------
    pub fn get_full_pb(&self, rel: PathBuf) -> PathBuf {
        let mut p = self.stat.get_untracked().db_path;
        p.push(rel);
        p
    }

    pub fn upd_stat(&self) {
        let stat = self.stat;
        let now = self.now;
        spawn_local(async move {
            match invoke("get_stat", &wasm_bindgen::JsValue::NULL).await {
                Ok(js) => {
                    let mut res = from_value::<StatisticsState>(js).unwrap_or_default();
                    res.demo_refs = res.demo_refs.map(|(name, desc)| {
                        let path = PathBuf::from("example").join(&name);
                        (path, desc)
                    });
                    stat.set(res);
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!("Error: {}", error_msg));
                }
            };
        });
    }

    pub fn remove_refer_ext(&self, p: &Path) -> String {
        let mut s = p.display().to_string();
        if s.ends_with(".refer") {
            s.truncate(s.len() - ".refer".len());
        }
        s
    }
}
