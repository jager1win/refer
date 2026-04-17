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

/*#[component]
pub fn Ref_el_edit() -> impl IntoView {
    let i18n = use_i18n();
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let data = RwSignal::new(None::<DataRecord>);
    let data_new = RwSignal::new(None::<DataRecord>);
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let edit_el = use_context::<RwSignal<bool>>().expect("edit_el not found");
    let sorted_keys = RwSignal::new(Vec::<(String, String, String, String)>::new());

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

                let for_keys: Vec<(String, String, String, String)> = transform_fields2(
                    &meta.get_untracked().unwrap().field_names,
                    &meta.get_untracked().unwrap().field_types,
                    &data.get_untracked().unwrap().fields,
                );
                sorted_keys.set(for_keys);

                data_new.set(Some(s))
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("{} {}", "Error:", &error_msg));
            }
        };
    });

    // name = "update" | "delete"
    let save = StoredValue::new_local(move |name: &str| {
        let action = name.to_string();
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        spawn_local(async move {
            match invoke(
                "apply_el_action",
                &tauri_args!(
                    "pb": pb,
                    "action" : action.clone(),
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
                    //selected.update(|c| c.refer = None);
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

            sorted_keys.get_untracked().into_iter().all(|(fi, _, ftype, _)| {
                if ftype == "number" {
                    let val = record.fields.get(&fi).cloned().unwrap_or_default();
                    // Разрешаем пустое поле или корректное число
                    val.is_empty() || val.parse::<f64>().is_ok()
                } else {
                    true // Текст валиден всегда
                }
            })
        })
    });

    let is_changed = Memo::new(move |_| data_new.get().unwrap().fields != data.get_untracked().unwrap().fields);

    let can_save = move || is_valid.get() && is_changed.get();

    Effect::new(move |_| {
        //log::info!("selected: {:#?}", selected.get());
        //log::debug!("orig {:?}", data.get());
        //log::debug!("curr {:?}", data_new.get());
        //log::debug!("is_changed {:?}",is_changed.get());
    });
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
                    let keys = sorted_keys.get();
                    keys.into_iter()
                        .map(|(fi, name, ftype, val)| {
                            let fi0 = fi.clone();
                            let fi1 = fi.clone();
                            let fi2 = fi.clone();
                            view! {
                                <label class="field-col">{name}<small>{ftype.clone()}</small></label>
                                <input
                                    type=ftype
                                    class:modified=move || {
                                        val != data_new.read().as_ref().and_then(|d| d.fields.get(&fi0)).cloned().unwrap_or_default()
                                    }
                                    prop:value=move || {
                                        data_new.read().as_ref().and_then(|d| d.fields.get(&fi1)).cloned().unwrap_or_default()
                                    }
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        data_new
                                            .update(|d| {
                                                if let Some(record) = d.as_mut() {
                                                    record.fields.insert(fi2.clone(), val);
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
}*/

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

    let info_list = RwSignal::new(meta.get_untracked().map(|m| m.info.clone()).unwrap_or_default());
    let fields_list = RwSignal::new(meta.get_untracked().map(|m| m.fields.clone()).unwrap_or_default());

    let save_info = move |_| {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        let new_info = info_list.get_untracked();

        spawn_local(async move {
            let _ = invoke(
                "update_meta_field",
                &tauri_args! { "pb": pb, "key": "info", "value": new_info },
            )
            .await;
        });
    };

    let save_fields = move |_| {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        let new_fields = fields_list.get_untracked();

        spawn_local(async move {
            let _ = invoke(
                "update_meta_field",
                &tauri_args! { "pb": pb, "key": "fields", "value": new_fields },
            )
            .await;
        });
    };
    Effect::new(move |_| {
        log::info!("fields_list: {:#?}", fields_list.get());
        //log::info!("selected {:?}", selected.get());
        //log::info!("ref_el data: {:#?}", data.get());
        //log::info!("sett: {:#?}", settings.get());
    });
    view! {
        // edit info(name & desc)
        <div class="gr center">
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
            <button class="m04" on:click=save_info>
                {t!(i18n, edit.save)}
            </button>
        </div>
        // edit exist fields
        <div class="gr center">
            <div class="grid2">
                {move || {
                    meta.get()
                        .unwrap()
                        .names
                        .into_iter()
                        .map(|field_key| {
                            let field_def = &fields_list.get()[&field_key];
                            let fkey = field_key.clone();
                            let fkey0 = field_key.clone();
                            view! {
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
                                    fields_list.update(|list| {
                                        if let Some(field_def) = list.get_mut(&fkey.clone()) {
                                            field_def.ftype = match field_def.ftype.as_str() {
                                                "text" => "number".to_string(),
                                                "number" => "text".to_string(),
                                                _ => "text".to_string(),
                                            };
                                        }
                                    });
                                }>
                                    {move || if fields_list.get()[&fkey0].ftype == "text" { "text 📝" } else { "number 🔢" }}
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <button class="m04" on:click=save_fields>
                {t!(i18n, edit.save)}
            </button>
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
