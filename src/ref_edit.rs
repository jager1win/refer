use crate::{app::*, i18n::*, state::State, tauri_args};
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Fields,
    Oper,
    Element,
}

#[component]
pub fn Ref_el_edit() -> impl IntoView {
    let i18n = use_i18n();
    let st = use_context::<State>().expect("State missing");
    let data = RwSignal::new(None::<DataRecord>);
    let data_new = RwSignal::new(None::<DataRecord>);
    let edit_el = use_context::<RwSignal<bool>>().expect("edit_el not found");

    // get el
    spawn_local(async move {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        match invoke("get_el", &tauri_args!("pb": pb, "id": Some(st.selected.get_untracked().id))).await {
            Ok(js) => {
                let s = from_value::<DataRecord>(js).unwrap();
                data.set(Some(s.clone()));
                data_new.set(Some(s));
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                st.now.set(format!("{} {}", "Error:", &error_msg));
            }
        };
    });

    let save = move |name: &str| {
        let action = name.to_string();
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        spawn_local(async move {
            match invoke(
                "apply_el_action",
                &tauri_args!("pb": pb,"action": action.clone(),"dr": data_new.get_untracked().unwrap(),),
            )
            .await
            {
                Ok(_s) => {
                    st.now.set(format!("{}: {}", &action, tu_string!(i18n, edit.saved)));
                    st.load_meta();
                    match action.as_str() {
                        "delete" => st.selected.update(|c| c.id = None),
                        "update" => edit_el.set(false),
                        &_ => todo!(),
                    };
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("{}: {:?}", &action, error_msg));
                }
            }
        });
    };

    let is_valid = Memo::new(move |_| {
        data_new.with(|maybe_record| {
            let record = match maybe_record {
                Some(r) => r,
                None => return false,
            };
            let meta_fields = &st.meta.get_untracked().unwrap().fields;

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
            <div class="header-row gr modified">
                <button on:click=move |_| edit_el.set(false)>"←"</button>

                <h5 class="m0 title-group">
                    "🔧 #" {st.selected.get_untracked().id.unwrap()} " | "
                    {move || st.selected.get_untracked().refer.unwrap().display().to_string()}
                </h5>
            </div>
            <div class="gr">
                <h6 class="modified">{t!(i18n, edit.warning_item_edit)}</h6>
                <hr />
                <h5>{t!(i18n, edit.title_fields_item)}</h5>
                <div class="grid2 a-start">
                    {move || {
                        let meta_fields = st.meta.get().unwrap().fields.clone();
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
            </div>
            <div class="gr bfc flex_wrap2 m04">
                <button on:click=move |_| save("delete")>
                    <span class="error">"🗑️ "{t!(i18n, all.del)}" "{t!(i18n, all.element)}</span>
                </button>
                <button disabled=move || !can_save() on:click=move |_| save("update")>
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
    let st = use_context::<State>().expect("State missing");
    let current_tab = RwSignal::new(None::<Tab>);

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
        pub struct Operation {
            pub id: u32,
            pub name: String,
            pub description: String,
            pub expression: String, // "f_6 * 17 / f_20"
            pub precision: u32,
        }
    3 группа Element
        добавить элемент */

    let del_ref = move |name: PathBuf| {
        spawn_local(async move {
            match invoke("del_ref", &tauri_args!("val" : name.clone())).await {
                Ok(_s) => {
                    st.now.set(format!("{}: {:?}", tu_string!(i18n, edit.ok_del_ref), &name));
                    st.selected.update(|c| c.refer = None);
                    st.edit_ref.set(false);
                    st.upd_stat();
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    st.now
                        .set(format!("{}: {:?} - {}", tu_string!(i18n, edit.er_del_ref), &name, error_msg));
                }
            }
        });
    };
    view! {
        <div class="header-row gr modified">
            <button on:click=move |_| st.edit_ref.set(false)>"←"</button>

            <h5 on:click=move |_| current_tab.set(None) class="m0 title-group sp_close">
                "🔧 "
                {move || st.selected.get_untracked().refer.unwrap().display().to_string()}
            </h5>
        </div>

        <nav class="gridline gr gap04">
            <button class:active=move || current_tab.get() == Some(Tab::Fields) on:click=move |_| current_tab.set(Some(Tab::Fields))>
                {t!(i18n, ref_main.columns)}
            </button>
            <button class:active=move || current_tab.get() == Some(Tab::Oper) on:click=move |_| current_tab.set(Some(Tab::Oper))>
                {t!(i18n, ref_main.operations)}
            </button>
            <button
                class:active=move || current_tab.get() == Some(Tab::Element)
                class="rr"
                on:click=move |_| current_tab.set(Some(Tab::Element))
            >
                {t!(i18n, all.element)}
            </button>
        </nav>

        {move || match current_tab.get() {
            None => {
                view! {
                    <div class="gr center">
                        <p class="warn pre-line">{t!(i18n, edit.warning)}</p>
                        <button role="button" class="error" on:click=move |_| del_ref(st.selected.get_untracked().refer.unwrap())>
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
    let st = use_context::<State>().expect("State missing");

    let info_list = RwSignal::new(Vec::<(String, String)>::new());
    let fields_list = RwSignal::new(HashMap::<String, FieldDef>::new());
    let new_fields = RwSignal::new(Vec::<(String, String)>::new());

    let original_info = RwSignal::new(Vec::<(String, String)>::new());
    let original_fields = RwSignal::new(HashMap::<String, FieldDef>::new());

    /*let get_meta = move || {
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
    };*/

    let upd_info = move |_| {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let new_info = info_list.get_untracked();

        spawn_local(async move {
            match invoke("update_meta_entity", &tauri_args! { "pb": pb, "key": "info", "value": new_info }).await {
                Ok(_s) => {
                    st.now.set(tu_string!(i18n, edit.saved).to_string());
                    st.load_meta()
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
    };

    let upd_fields = move |_| {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let updated_fields = fields_list.get_untracked();

        spawn_local(async move {
            match invoke(
                "update_meta_entity",
                &tauri_args! { "pb": pb, "key": "fields", "value": updated_fields },
            )
            .await
            {
                Ok(_s) => {
                    st.now.set(tu_string!(i18n, edit.saved).to_string());
                    st.load_meta()
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
    };

    let del_field = move |f: String| {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());

        spawn_local(async move {
            match invoke("del_field", &tauri_args! { "pb": pb, "index": f}).await {
                Ok(_s) => {
                    st.now.set(tu_string!(i18n, edit.saved).to_string());
                    st.load_meta()
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
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
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let new_f: Vec<(String, String)> = new_fields.get_untracked();

        spawn_local(async move {
            match invoke("add_fields", &tauri_args! { "pb": pb, "fields": new_f }).await {
                Ok(_s) => {
                    st.now.set(tu_string!(i18n, edit.saved).to_string());
                    st.load_meta()
                    //new_fields.set(Vec::<(String, String)>::new());
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
    };

    let info_modified = Memo::new(move |_| info_list.get() != original_info.get());
    let fields_modified = Memo::new(move |_| fields_list.get() != original_fields.get());
    let new_fields_modified = Memo::new(move |_| !new_fields.get().is_empty());

    Effect::new(move |_| {
        //log::info!("new_fields: {:#?}", new_fields.get());
        //log::info!("meta {:?}", st.meta.get().unwrap());
        if let Some(m) = st.meta.get() {
            info_list.set(m.info.clone());
            fields_list.set(m.fields.clone());
            original_info.set(m.info.clone());
            original_fields.set(m.fields.clone());
        }
    });
    view! {
        // edit info(name & desc) // class="bf"
        <div class="gr center">
            <p class="center warn">{t!(i18n, edit.meta_hint)}</p>
            <div class="grid1a">
                {move || {
                    info_list
                        .get()
                        .iter()
                        .enumerate()
                        .map(|(idx, (label, value))| {
                            view! {
                                <label>
                                    {match label.as_str() {
                                        "name" => view! { {t!(i18n, ref_main.name)} }.into_any(),
                                        "desc" => view! { {t!(i18n, ref_main.desc)} }.into_any(),
                                        &_ => view! { "" }.into_any(),
                                    }}
                                </label>
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
            <button class=move || if info_modified.get() { "modified m04" } else { "m04" } on:click=upd_info>
                "💾 "
                {t!(i18n, edit.save)}
            </button>
        </div>
        // edit exist fields
        <Show when=move || !fields_list.get().is_empty() fallback=|| view! { <div>""</div> }>
            <div class="gr center">
                <p class="center warn">{t!(i18n, edit.fields_edit_hint)}</p>
                <div class="grida1a">
                    {move || {
                        st.meta
                            .get()
                            .unwrap()
                            .names
                            .into_iter()
                            .map(|field_key| {
                                let field_def = &fields_list.get()[&field_key];
                                let fkey = field_key.clone();
                                let fkey1 = field_key.clone();
                                view! {
                                    <button
                                        class=""
                                        on:click=move |_| {
                                            fields_list
                                                .update(|list| {
                                                    list.remove(&fkey1);
                                                });
                                            st.meta
                                                .update(|m| {
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
                                    }>{field_def.ftype.clone()}</button>
                                }
                            })
                            .collect_view()
                    }}
                </div>

                <button class=move || if fields_modified.get() { "m04 bf modified" } else { "m04 bf" } on:click=upd_fields>
                    "💾 "
                    {t!(i18n, edit.save)}
                </button>
            </div>
        </Show>

        // add fields
        <div class="gr">
            <p class="center warn">{t!(i18n, edit.fields_add_hint)}</p>
            <div class="grid2">
                <For
                    each=move || new_fields.get().into_iter().enumerate()
                    key=|(idx, _)| *idx
                    children=move |(idx, (val, _kind))| {
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
            <div class="flex-center m04">
                <button on:click=add_new_field>"✚ "{t!(i18n, edit.add)}</button>
                {move || match new_fields.get().is_empty() {
                    true => view! { "" }.into_any(),
                    false => {
                        view! {
                            <button on:click=move |_| new_fields.set(Vec::<(String, String)>::new())>"🧹 "{t!(i18n, all.clear)}</button>

                            <button class=move || if new_fields_modified.get() { "modified" } else { "" } on:click=save_new_fields>
                                "💾 "
                                {t!(i18n, edit.save)}
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/*#[component]
pub fn ElementCrud() -> impl IntoView {
    let i18n = use_i18n();
    let st = use_context::<State>().expect("State missing");
    let add_mode = RwSignal::new(true);
    let saved_id = RwSignal::new(None::<u32>);
    let form_data: RwSignal<HashMap<String, String>> = RwSignal::new(HashMap::<String, String>::new());

    let save_new = move |_| {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let data = form_data.get_untracked();

        spawn_local(async move {
            match invoke("add_element", &tauri_args! { "pb": pb, "fields": data }).await {
                Ok(js) => {
                    let id = from_value::<u32>(js).unwrap();
                    saved_id.set(Some(id));
                    add_mode.set(false);
                    form_data.set(HashMap::new()); // очищаем для следующего раза
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            }
        });
    };

    Effect::new(move |_| {
        //log::info!("edit_ref {:?}", edit_ref.get());
    });
    view! {
        {move || match add_mode.get() {
            false => {
                view! {
                    <div class="gr grid center">
                        <button on:click=move |_| {
                            st.edit_ref.set(false);
                            st.selected.update(|s| s.id = saved_id.get_untracked())
                        }>"→ "{t!(i18n, all.element)}</button>
                        <button on:click=move |_| st.edit_ref.set(false)>"→ "{t!(i18n, all.reference)}</button>
                        <button on:click=move |_| add_mode.set(true)>"+ "{t!(i18n, all.element)}</button>
                    </div>
                }
                    .into_any()
            }
            true => {
                view! {
                    <div class="grid2 gr a-start">
                        {move || {
                            st.meta
                                .get()
                                .unwrap()
                                .names
                                .iter()
                                .map(|field_key| {
                                    let binding = st.meta.get().unwrap();
                                    let field_def = binding.fields.get(field_key).unwrap();
                                    let field_key = field_key.clone();
                                    let value = form_data.get().get(&field_key).cloned().unwrap_or_default();

                                    view! {
                                        <label class="field-col">{field_def.name.clone()}<small>{field_def.ftype.clone()}</small></label>
                                        <input
                                            type=field_def.ftype.clone()
                                            inputmode=if field_def.ftype == "number" { "decimal" } else { "text" }
                                            prop:value=value
                                            on:input=move |ev| {
                                                let val = event_target_value(&ev);
                                                form_data
                                                    .update(|f| {
                                                        f.insert(field_key.clone(), val);
                                                    });
                                            }
                                        />
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                    <div class="gr center">
                        <button on:click=save_new>"💾 "{t!(i18n, edit.save)}</button>
                    </div>
                }
                    .into_any()
            }
        }}
    }
}*/

#[component]
pub fn ElementCrud() -> impl IntoView {
    let i18n = use_i18n();
    let st = use_context::<State>().expect("State missing");
    let add_mode = RwSignal::new(true);
    let saved_id = RwSignal::new(None::<u32>);

    // Создаем начальную структуру с пустыми значениями для всех полей
    let init_form_data = move || {
        let mut data = HashMap::<String, String>::new();
        if let Some(meta) = st.meta.get_untracked() {
            for field_key in &meta.names {
                data.insert(field_key.clone(), String::new());
            }
        }
        data
    };

    let form_data: RwSignal<HashMap<String, String>> = RwSignal::new(init_form_data());

    let save_new = move |_| {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let data = form_data.get_untracked();

        // Опционально: фильтруем только непустые значения
        /*let filtered_data: HashMap<String, String> = data.clone()
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .collect();*/

        spawn_local(async move {
            // Используйте data для отправки всех полей или filtered_data для только заполненных
            match invoke("add_element", &tauri_args! { "pb": pb, "fields": data }).await {
                Ok(js) => {
                    let id = from_value::<u32>(js).unwrap();
                    saved_id.set(Some(id));
                    add_mode.set(false);
                    form_data.set(init_form_data()); // Сбрасываем с пустыми значениями вместо пустого HashMap
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            }
        });
    };

    Effect::new(move |_| {
        //log::info!("edit_ref {:?}", edit_ref.get());
    });

    view! {
        {move || match add_mode.get() {
            false => {
                view! {
                    <div class="gr grid center">
                        <button on:click=move |_| {
                            st.edit_ref.set(false);
                            st.selected.update(|s| s.id = saved_id.get_untracked())
                        }>"→ "{t!(i18n, all.element)}</button>
                        <button on:click=move |_| st.edit_ref.set(false)>"→ "{t!(i18n, all.reference)}</button>
                        <button on:click=move |_| {
                            form_data.set(init_form_data());
                            add_mode.set(true);
                        }>"+ "{t!(i18n, all.element)}</button>
                    </div>
                }
                    .into_any()
            }
            true => {
                view! {
                    <div class="gr">
                        <p class="center warn">{t!(i18n, edit.add_item_hint)}</p>
                        <div class="grid2 a-start">
                            <small class="center">{t!(i18n, ref_main.column_name)}", "{t!(i18n, ref_main.column_type)}</small>
                            <small class="center">{t!(i18n, ref_main.column_value)}</small>
                            {move || {
                                st.meta
                                    .get()
                                    .unwrap()
                                    .names
                                    .iter()
                                    .map(|field_key| {
                                        let binding = st.meta.get().unwrap();
                                        let field_def = binding.fields.get(field_key).unwrap();
                                        let field_key = field_key.clone();
                                        let value = form_data.get().get(&field_key).cloned().unwrap_or_default();

                                        view! {
                                            <label class="field-col">
                                                {field_def.name.clone()}<small>{field_def.ftype.clone()}</small>
                                            </label>
                                            <input
                                                type=field_def.ftype.clone()
                                                inputmode=if field_def.ftype == "number" { "decimal" } else { "text" }
                                                prop:value=value
                                                on:input=move |ev| {
                                                    let val = event_target_value(&ev);
                                                    form_data
                                                        .update(|f| {
                                                            f.insert(field_key.clone(), val);
                                                        });
                                                }
                                            />
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </div>
                    <div class="gr center">
                        <button on:click=save_new>"💾 "{t!(i18n, edit.add)}</button>
                    </div>
                }
                    .into_any()
            }
        }}
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Scene {
    List,
    Edit(usize), // Индекс в массиве для редактирования
    Create,      // Отдельный режим для новой записи
}

#[component]
pub fn OperCrud() -> impl IntoView {
    let i18n = use_i18n();
    let st = use_context::<State>().expect("State missing");
    let scene = RwSignal::new(Scene::List);

    let del_oper = move |id: u32| {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let new_oper_list = st
            .meta
            .get_untracked()
            .unwrap()
            .operations
            .into_iter()
            .filter(|f| f.id != id)
            .collect::<Vec<Operation>>();

        spawn_local(async move {
            match invoke(
                "update_meta_entity",
                &tauri_args! { "pb": pb, "key": "operations", "value": new_oper_list },
            )
            .await
            {
                Ok(_s) => {
                    st.now.set(tu_string!(i18n, edit.saved).to_string());
                    st.load_meta();
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
    };

    view! {
        <div class="gr center">
            {move || match scene.get() {
                Scene::List => {
                    Either::Left(
                        view! {
                            <div class="grida1a">
                                <For
                                    each=move || st.meta.get().unwrap().operations.into_iter().enumerate()
                                    key=|(_, op)| leptos::serde_json::to_string(&op).unwrap_or_else(|_| format!("o_{}", op.id))
                                    children=move |(idx, op)| {
                                        view! {
                                            <button on:click=move |_| del_oper(op.id)>"🗑️"</button>
                                            <span class="grid gap02">
                                                {op.name}" " {(!op.desc.is_empty()).then(|| format!("({})", op.desc))}
                                                <small>{op.expr}</small>
                                            </span>
                                            <button on:click=move |_| scene.set(Scene::Edit(idx))>"✏️"</button>
                                        }
                                    }
                                />
                            </div>
                            <hr />
                            <button on:click=move |_| scene.set(Scene::Create)>"+ "{t!(i18n, edit.add)}</button>
                        },
                    )
                }
                Scene::Edit(idx) => {
                    Either::Right(
                        view! {
                            <EditOper
                                // Берем конкретную операцию по индексу
                                initial_data=Some(st.meta.get().unwrap().operations[idx].clone())
                                on_done=Callback::new(move |_| {
                                    scene.set(Scene::List);
                                    st.load_meta();
                                })
                            />
                        },
                    )
                }
                Scene::Create => {
                    Either::Right(
                        view! {
                            <EditOper
                                initial_data=None
                                on_done=Callback::new(move |_| {
                                    scene.set(Scene::List);
                                    st.load_meta();
                                })
                            />
                        },
                    )
                }
            }}
        </div>
    }
}

#[component]
pub fn EditOper(initial_data: Option<Operation>, #[prop(into)] on_done: Callback<()>) -> impl IntoView {
    use exmex::Express;
    let i18n = use_i18n();
    let st = use_context::<State>().expect("State missing");
    let op = RwSignal::new(initial_data.clone().unwrap_or(Operation {
        id: 0,
        name: "New Operation".into(),
        desc: "Description".into(),
        expr: "({Путь}/{道}) + {이} + Αρχη * {كل} / {것}".into(), // Путь — это источник всех вещей. Лао-цзы
        prec: 2,
    }));

    let vars = st
        .meta
        .get_untracked()
        .unwrap()
        .fields
        .into_iter()
        .filter(|f| f.1.ftype == "number")
        .map(|f| {
            let random_f64 = js_sys::Math::random();
            let val = (random_f64 * 91.0).floor() + 20.0;
            let final_val = val / 10.0;
            (f.1.name, final_val)
        })
        .collect::<HashMap<String, f64>>();
    let (inputs, set_inputs) = signal(vars.clone());

    // Парсинг формулы
    let expr_result = Memo::new(move |_| exmex::parse::<f64>(&op.get().expr));

    fn display_name(raw: &str) -> String {
        raw.trim_matches(|c| c == '{' || c == '}').to_string()
    }

    let var_names = Memo::new(move |_| {
        match expr_result.get() {
            Ok(e) => e
                .var_names()
                .iter()
                .map(|n| display_name(n)) // " {x} " -> " x "
                .collect::<Vec<_>>(),
            Err(_) => vec![],
        }
    });

    let calculation = Memo::new(move |_| {
        match expr_result.get() {
            Ok(e) => {
                let current_inputs = inputs.get();
                let mut vals = Vec::new();

                for raw_name in e.var_names() {
                    // Извлекаем "чистое" имя, чтобы найти его в нашем словаре инпутов
                    let clean = display_name(raw_name);
                    if let Some(val) = current_inputs.get(&clean) {
                        vals.push(*val);
                    } else {
                        return Err(format!("Введите значение для {}", clean));
                    }
                }

                e.eval(&vals).map(|v| format!("{:.4}", v)).map_err(|e| format!("Ошибка: {:?}", e))
            }
            Err(e) => Err(format!("Формула: {}", e)),
        }
    });

    // Функция очистки
    /*let clear_all = move |_| {
        //set_formula.set(String::new());
        //set_inputs.update(|map| map.clear());
    };*/

    let filtered = move || {
        let names = var_names.get();
        let inputs = vars.clone();
        names.into_iter().filter(|n| !inputs.contains_key(n)).collect::<Vec<String>>()
    };

    let save = move |_| {
        let op = op.get_untracked();
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());

        spawn_local(async move {
            match invoke("save_oper", &tauri_args! { "pb": pb, "oper": op }).await {
                Ok(_s) => {
                    st.now.set(tu_string!(i18n, edit.saved).to_string());
                    st.load_meta();
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            };
        });
        on_done.run(());
    };

    Effect::new(move |_| {
        //log::info!("new_fields: {:#?}", new_fields.get());
        //og::info!("meta {:?}", meta.get());
        log::debug!("var_names:{:?}", var_names.get());
        log::debug!("inputs:{:?}", inputs.get());
        log::debug!("op:{:?}", op.get());
    });

    view! {
        <div class="oper grid center">
            <div>
                <p class="warn">
                    "Expression"<br />
                    "Variables should consist only of latin or greek letters, numbers, and underscores."
                    "They need to fit the regular expression r\"[a-zA-Zα-ωΑ-Ω_]+[a-zA-Zα-ωΑ-Ω_0-9]*\", if they are not between curly brackets."
                </p>
                <input
                    class="w100 center"
                    type="text"
                    prop:value=move || op.get().expr
                    on:input=move |ev| op.update(|o| o.expr = event_target_value(&ev))
                    attr:data-error=move || expr_result.get().is_err()
                />
            </div>

            // блок числовых полей со случайными значениями
            <div class="ch_m">
                {move || {
                    inputs
                        .get()
                        .into_iter()
                        .map(|inp| {
                            let inp_name = inp.clone().0;
                            let inpclon = inp.clone();
                            view! {
                                <button on:click=move |_| {
                                    op.update(|f| {
                                        if !f.expr.is_empty() && !f.expr.ends_with(' ') {
                                            f.expr.push(' ');
                                        }
                                        f.expr.push_str(&inp_name);
                                        f.expr.push(' ');
                                    });
                                }>{inpclon.0}"("{inpclon.1}")"</button>
                            }
                        })
                        .collect_view()
                }}
            </div>

            // блок операторов
            <div class="ch_m">
                {get_standard_operators()
                    .into_iter()
                    .map(|ops| {
                        view! {
                            <button on:click=move |_| {
                                op.update(|f| {
                                    if !f.expr.is_empty() && !f.expr.ends_with(' ') {
                                        f.expr.push(' ');
                                    }
                                    f.expr.push_str(ops);
                                    f.expr.push(' ');
                                });
                            }>{ops}</button>
                        }
                    })
                    .collect_view()}
            </div>

            // результат и ошибки
            {move || match calculation.get() {
                Ok(res) => view! { <div data-status="ok">" = " {res}</div> }.into_any(),
                Err(err) => view! { <div data-status="error">{err}</div> }.into_any(),
            }}

            // Динамические инпуты для переменных
            <section class="grid3">
                <For
                    each=move || filtered()
                    key=|name| name.clone()
                    children=move |name| {
                        let n_label = name.clone();
                        let n_input = name.clone();
                        view! {
                            <div>
                                <label>{n_label}</label>
                                <input
                                    type="text"
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev).parse::<f64>().unwrap_or(0.0);
                                        set_inputs
                                            .update(|map| {
                                                map.insert(n_input.clone(), val);
                                            });
                                    }
                                    prop:value=move || inputs.get().get(&name).cloned().unwrap_or(0.0)
                                />
                            </div>
                        }
                    }
                />
            </section>
            <section class="flex-center">
                <button on:click=move |_| on_done.run(())>{t!(i18n, edit.cancel)}</button>
                <button on:click=save>{t!(i18n, edit.save)}</button>
            </section>
        </div>
    }
}

pub fn get_standard_operators() -> Vec<&'static str> {
    vec![
        "(", ")", "{", "}", // скобки
        "+", "-", "*", "/", "^", "%", // Арифметика
        "sin", "cos", "tan", "asin", "acos", // Тригонометрия
        "atan", "atan2", "sinh", "cosh", "tanh", "exp", "ln", "log10", "log2", "sqrt", // Логарифмы и корни
        "abs", "signum", "floor", "ceil", "round", "PI", "TAU", "E", // Константы
    ]
}

/*

        <button on:click=move |_| { edit_ref.set(false) }>"💾 "{t!(i18n, edit.save)}</button>
        <button on:click=move |_| del_ref(selected.get_untracked().refer.unwrap())>"🗑️ "{t!(i18n, all.del)}</button>

*/
/*
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
*/
