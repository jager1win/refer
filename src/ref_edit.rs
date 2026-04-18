use crate::{app::*, functions::*, i18n::*, tauri_args};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Element,
    Fields,
    Oper,
}

#[component]
pub fn Ref_el_edit() -> impl IntoView {
    let i18n = use_i18n();
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let data = RwSignal::new(None::<DataRecord>);
    let data_new = RwSignal::new(None::<DataRecord>);
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let edit_el = use_context::<RwSignal<bool>>().expect("edit_el not found");

    // get el
    spawn_local(async move {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        match invoke(
            "get_el",
            &tauri_args!("pb": pb, "id": Some(selected.get_untracked().id)),
        )
        .await
        {
            Ok(js) => {
                let s = from_value::<DataRecord>(js).unwrap();
                data.set(Some(s.clone()));
                data_new.set(Some(s));
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("{} {}", "Error:", &error_msg));
            }
        };
    });

    let save = StoredValue::new_local(move |name: &str| {
        let action = name.to_string();
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        spawn_local(async move {
            match invoke(
                "apply_el_action",
                &tauri_args!(
                    "pb": pb,
                    "action": action.clone(),
                    "dr": data_new.get_untracked().unwrap(),
                ),
            )
            .await
            {
                Ok(_s) => {
                    now.set(format!("{}: {}", &action, tu_string!(i18n, edit.saved)));
                    match action.as_str() {
                        "delete" => selected.update(|c| c.id = None),
                        "update" => edit_el.set(false),
                        &_ => todo!(),
                    };
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    now.set(format!("{}: {:?}", &action, error_msg));
                }
            }
        });
    });

    let is_valid = Memo::new(move |_| {
        data_new.with(|maybe_record| {
            let record = match maybe_record {
                Some(r) => r,
                None => return false,
            };
            let meta_fields = &meta.get_untracked().unwrap().fields;

            meta_fields.iter().all(|(field_key, field_def)| {
                if field_def.ftype == "number" {
                    let val = record.fields.get(field_key).cloned().unwrap_or_default();
                    val.is_empty() || val.parse::<f64>().is_ok()
                } else {
                    true
                }
            })
        })
    });

    let is_changed = Memo::new(move |_| data_new.get().unwrap().fields != data.get_untracked().unwrap().fields);

    let can_save = move || is_valid.get() && is_changed.get();

    view! {
        <Show when=move || data_new.read().is_some() fallback=|| view! { <progress /> }>
            <div class="header-row gr">
                <button on:click=move |_| edit_el.set(false)>"←"</button>
                <h5 class="m0 title-group">
                    "🔧 ID " {selected.get_untracked().id.unwrap()} " | " {format!("{:?}", selected.get_untracked().refer.unwrap())}
                </h5>
            </div>
            <div class="grid1a gr a-start">
                {move || {
                    let meta_fields = meta.get().unwrap().fields.clone();
                    let data_new_val = data_new.get().unwrap();
                    meta_fields
                        .iter()
                        .map(|(field_key, field_def)| {
                            let field_key_clone = field_key.clone();
                            let field_key_clone2 = field_key.clone();
                            let current_val = data_new_val.fields.get(field_key).cloned().unwrap_or_default();

                            view! {
                                <label class="field-col">{field_def.name.clone()} <small>{field_def.ftype.clone()}</small></label>
                                <input
                                    type=field_def.ftype.clone()
                                    class:modified=move || {
                                        data_new.get().unwrap().fields.get(&field_key_clone).cloned().unwrap_or_default()
                                            != data.get_untracked().unwrap().fields.get(&field_key_clone).cloned().unwrap_or_default()
                                    }
                                    prop:value=current_val
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        data_new
                                            .update(|d| {
                                                if let Some(record) = d.as_mut() {
                                                    record.fields.insert(field_key_clone2.clone(), val);
                                                }
                                            });
                                    }
                                />
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div class="gr bfc flex_wrap3 m04">
                <button on:click=move |_| save.with_value(|a| a("delete"))>
                    <span class="error">"🗑️ "{t!(i18n, all.del)}</span>
                </button>
                <button disabled=move || !can_save() on:click=move |_| save.with_value(|a| a("update"))>
                    "💾 "
                    {t!(i18n, edit.save)}
                </button>
            </div>
        </Show>
    }
}

#[component]
pub fn Ref_edit() -> impl IntoView {
    let i18n = use_i18n();
    //let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("edit not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let active_tab = RwSignal::new(None::<Tab>);

    /*
        - CRUD элемента - только добавление одного нового
        - CRUD полей
        - CRUD операций(плюс тесты сразу на ошибки - первые 10 элементов или случайных)
        - CRUD meta
    1 группа Fields
        name:
        desc:
        field_names:
        field_types:
        добавить поля
    2 группа Oper
        operations: [
            Operation {
                name: "Energy (J)",
                description: "Kinetic energy in Joules",
                expression: "f_1 * f_2 * f_2 / 2000",
                precision: 2,
            },
    3 группа Element
        добавить элемент */

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
        <div class="header-row gr">
            <button on:click=move |_| edit_ref.set(false)>"←"</button>

            <nav class="gap04">
                <button class:active=move || active_tab.get() == Some(Tab::Fields) on:click=move |_| active_tab.set(Some(Tab::Fields))>
                    {t!(i18n, ref_main.columns)}
                </button>
                <button class:active=move || active_tab.get() == Some(Tab::Oper) on:click=move |_| active_tab.set(Some(Tab::Oper))>
                    {t!(i18n, ref_main.operations)}
                </button>
                <button class:active=move || active_tab.get() == Some(Tab::Element) on:click=move |_| active_tab.set(Some(Tab::Element))>
                    "✚"
                    {t!(i18n, all.element)}
                </button>
            </nav>

            <div on:click=move |_| active_tab.set(None) class="title-group sp_close">
                "🔧 "
                <span>{move || remove_refer_ext(&selected.get().refer.unwrap())}</span>
            </div>
        </div>

        {move || match active_tab.get() {
            None => {
                view! {
                    <div class="gr center">
                        <p class="warn pre-line">{t!(i18n, edit.warning)}</p>
                        <button role="button" class="error" on:click=move |_| del_ref(selected.get_untracked().refer.unwrap())>
                            "🗑️ "
                            {t!(i18n, all.del)}
                            " "
                            {t!(i18n,all.reference)}
                        </button>
                    </div>
                }
                    .into_any()
            }
            Some(Tab::Fields) => view! { <FieldsCrud /> }.into_any(),
            Some(Tab::Oper) => view! { <OperCrud /> }.into_any(),
            Some(Tab::Element) => view! { <ElementCrud /> }.into_any(),
        }}
    }
}

#[component]
pub fn FieldsCrud() -> impl IntoView {
    let i18n = use_i18n();
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");

    let info_list = RwSignal::new(Vec::<(String, String)>::new());
    let fields_list = RwSignal::new(HashMap::<String, FieldDef>::new());
    let new_fields = RwSignal::new(Vec::<(String, String)>::new());

    let original_info = RwSignal::new(Vec::<(String, String)>::new());
    let original_fields = RwSignal::new(HashMap::<String, FieldDef>::new());

    let get_meta = move || {
        spawn_local(async move {
            let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
            match invoke("get_meta", &tauri_args!("pb": pb)).await {
                Ok(js) => {
                    let s = from_value::<TableMeta>(js).unwrap();
                    meta.set(Some(s))
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!("Error: {}", error_msg));
                }
            };
        });
    };

    let upd_info = move |_| {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        let new_info = info_list.get_untracked();

        spawn_local(async move {
            match invoke(
                "update_meta_entity",
                &tauri_args! { "pb": pb, "key": "info", "value": new_info },
            )
            .await
            {
                Ok(_s) => {
                    now.set(tu_string!(i18n, edit.saved).to_string());
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
        get_meta();
    };

    let upd_fields = move |_| {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        let updated_fields = fields_list.get_untracked();

        spawn_local(async move {
            match invoke(
                "update_meta_entity",
                &tauri_args! { "pb": pb, "key": "fields", "value": updated_fields },
            )
            .await
            {
                Ok(_s) => {
                    now.set(tu_string!(i18n, edit.saved).to_string());
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
        get_meta();
    };

    let del_field = move |f: String| {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());

        spawn_local(async move {
            match invoke("del_field", &tauri_args! { "pb": pb, "index": f}).await {
                Ok(_s) => {
                    now.set(tu_string!(i18n, edit.saved).to_string());
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
        get_meta();
    };

    let add_new_field = move |_| {
        new_fields.update(move |f| {
            f.push(("Field".to_string(), "text".to_string()));
        });
    };

    let toggle_type = move |idx: usize| {
        new_fields.update(|f| {
            if let Some(field) = f.get_mut(idx) {
                field.1 = if field.1 == "text" {
                    "number".to_string()
                } else {
                    "text".to_string()
                };
            }
        });
    };

    let save_new_fields = move |_| {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        let new_f: Vec<(String, String)> = new_fields.get_untracked();

        spawn_local(async move {
            match invoke("add_fields", &tauri_args! { "pb": pb, "fields": new_f }).await {
                Ok(_s) => {
                    now.set(tu_string!(i18n, edit.saved).to_string());
                    new_fields.set(Vec::<(String, String)>::new());
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    now.set(format!("Error: {:?}", error_msg));
                }
            };
        });

        get_meta();
    };

    let info_modified = Memo::new(move |_| info_list.get() != original_info.get());
    let fields_modified = Memo::new(move |_| fields_list.get() != original_fields.get());
    let new_fields_modified = Memo::new(move |_| !new_fields.get().is_empty());

    Effect::new(move |_| {
        log::info!("new_fields: {:#?}", new_fields.get());
        log::info!("meta {:?}", meta.get());
        if let Some(m) = meta.get() {
            info_list.set(m.info.clone());
            fields_list.set(m.fields.clone());
            original_info.set(m.info.clone());
            original_fields.set(m.fields.clone());
        }
    });
    view! {
        // edit info(name & desc)
        <div class="gr grid41">
            <div class="grid1a">
                {move || {
                    info_list
                        .get()
                        .iter()
                        .enumerate()
                        .map(|(idx, (label, value))| {
                            view! {
                                <label>{label.clone()}</label>
                                <input
                                    type="text"
                                    prop:value=value.clone()
                                    on:input=move |ev| {
                                        info_list.update(|list| list[idx].1 = event_target_value(&ev));
                                    }
                                />
                            }
                        })
                        .collect_view()
                }}
            </div>
            <button class=move || if info_modified.get() { "modified" } else { "" } on:click=upd_info>
                {t!(i18n, edit.save)}
            </button>
        </div>
        // edit exist fields
        <Show when=move || !fields_list.get().is_empty() fallback=|| view! { <div>""</div> }>
            <div class="gr grid41">
                <div class="grid3">
                    {move || {
                        meta.get()
                            .unwrap()
                            .names
                            .into_iter()
                            .map(|field_key| {
                                let field_def = &fields_list.get()[&field_key];
                                let fkey = field_key.clone();
                                let fkey0 = field_key.clone();
                                let fkey1 = field_key.clone();
                                view! {
                                    <button
                                        class=""
                                        on:click=move |_| {
                                            fields_list
                                                .update(|list| {
                                                    list.remove(&fkey1);
                                                });
                                            meta.update(|m| {
                                                if let Some(meta_data) = m {
                                                    meta_data.fields.remove(&fkey1);
                                                    meta_data.names.retain(|n| n != &fkey1);
                                                }
                                            });
                                            del_field(fkey1.clone());
                                        }
                                    >
                                        "🗑️"
                                    </button>

                                    <input
                                        type="text"
                                        prop:value=field_def.name.clone()
                                        on:input=move |ev| {
                                            fields_list
                                                .update(|list| {
                                                    if let Some(field_def) = list.get_mut(&field_key) {
                                                        field_def.name = event_target_value(&ev);
                                                    }
                                                });
                                        }
                                    />

                                    <button on:click=move |_| {
                                        fields_list
                                            .update(|list| {
                                                if let Some(field_def) = list.get_mut(&fkey.clone()) {
                                                    field_def.ftype = match field_def.ftype.as_str() {
                                                        "text" => "number".to_string(),
                                                        "number" => "text".to_string(),
                                                        _ => "text".to_string(),
                                                    };
                                                }
                                            });
                                    }>
                                        {move || { if fields_list.get()[&fkey0].ftype == "text" { "text 📝" } else { "number 🔢" } }}
                                    </button>
                                }
                            })
                            .collect_view()
                    }}
                </div>
                <button class=move || if fields_modified.get() { "modified" } else { "" } on:click=upd_fields>
                    {t!(i18n, edit.save)}
                </button>
            </div>
        </Show>

        // add fields
        <div class="gr grid41">
            <div class="grid">
                <div class="grid2">
                    <For
                        each=move || new_fields.get().into_iter().enumerate()
                        key=|(idx, _)| *idx
                        children=move |(idx, (val, kind))| {
                            view! {
                                <input
                                    type="text"
                                    placeholder=format!("Field {}", idx + 1)
                                    prop:value=val
                                    on:input=move |ev| {
                                        let new_val = event_target_value(&ev);
                                        new_fields
                                            .update(|f| {
                                                if let Some(field) = f.get_mut(idx) {
                                                    field.0 = new_val;
                                                }
                                            });
                                    }
                                />
                                <button on:click=move |_| toggle_type(
                                    idx,
                                )>{move || new_fields.get().get(idx).map(|(_, k)| k.clone()).unwrap_or_default()}</button>
                            }
                        }
                    />
                </div>
            </div>
            <div class="grid">
                <button on:click=add_new_field>{t!(i18n, edit.add_field)}</button>

                <button on:click=move |_| new_fields.set(Vec::<(String, String)>::new())>{t!(i18n, all.clear)}</button>

                <button class=move || if new_fields_modified.get() { "modified" } else { "" } on:click=save_new_fields>
                    {t!(i18n, edit.save)}
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn OperCrud() -> impl IntoView {
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    view! { <>"Oper"</> }
}

#[component]
pub fn ElementCrud() -> impl IntoView {
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    view! { <>"Element"</> }
}

/*

        <button on:click=move |_| { edit_ref.set(false) }>"💾 "{t!(i18n, edit.save)}</button>
        <button on:click=move |_| del_ref(selected.get_untracked().refer.unwrap())>"🗑️ "{t!(i18n, all.del)}</button>

*/
/*
    let save_search_config = move |field: String| {
        let Some(mut meta_data) = meta.get_untracked() else {
            return;
        };
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        if let Some(pos) = meta_data.search_config.iter().position(|f| f == &field) {
            meta_data.search_config.remove(pos);
        } else {
            meta_data.search_config.push(field);
        }
        meta.set(Some(meta_data.clone()));

        spawn_local(async move {
            let _ = invoke(
                "update_meta_field",
                &tauri_args! {
                    "pb": pb,
                    "key": "search_config",
                    "value": meta_data.search_config
                },
            )
            .await;
        });
    };
*/
