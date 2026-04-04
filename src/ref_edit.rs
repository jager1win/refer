use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::path::PathBuf;
use crate::{app::*, functions::*, i18n::*, tauri_args};

#[component]
pub fn Ref_edit() -> impl IntoView {
    let i18n = use_i18n();
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("edit not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");

    let del_ref = move |name: PathBuf| {
        spawn_local(async move {
            match invoke("del_ref", &tauri_args!("val" : name.clone())).await {
                Ok(_s) => {
                    now.set(format!("{}: {:?}", tu_string!(i18n, edit.ok_del_ref), &name));
                    selected.update(|c| c.refer = None);
                    edit_ref.set(false);
                    upd_stat(stat, now);
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!(
                        "{}: {:?} - {}",
                        tu_string!(i18n, edit.er_del_ref),
                        &name,
                        error_msg
                    ));
                }
            }
        });
    };
    view! {
        <h3>{t!(i18n, edit.test)}</h3>

        <button on:click=move |_| { edit_ref.set(false) }>"✎ Save "</button>
        <button on:click=move |_| del_ref(selected.get_untracked().refer.unwrap())>{t!(i18n, all.del)}</button>
    }
}

#[component]
pub fn Ref_el_edit() -> impl IntoView {
    view! { "Ref_el_edit  ыва к" }
}