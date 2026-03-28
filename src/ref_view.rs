use crate::{app::*, functions::*, i18n::*, tauri_args};
use core::f64;
use exmex::{parse, prelude::*};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct RefState {
    // -1,0,1
    meta: i8,
    data: i8,
}
impl RefState {
    fn new() -> Self {
        Self { meta: 0, data: 0 }
    }
}

#[component]
pub fn Ref_main() -> impl IntoView {
    let i18n = use_i18n();
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("reference not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
    let query_string = RwSignal::new("".to_string());
    let meta = RwSignal::new(None::<TableMeta>);
    let data = RwSignal::new(None::<Vec<DataRecord>>);
    let selected_el = RwSignal::new(None::<u32>);
    let ref_state = RwSignal::new(RefState::new());

    // get meta
    spawn_local(async move {
        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
        match invoke("get_meta", &tauri_args!("pb": pb)).await {
            Ok(js) => {
                let s = from_value::<TableMeta>(js).unwrap();
                ref_state.update(|st| {
                    st.meta = 1;
                });
                meta.set(Some(s))
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                ref_state.update(|st| {
                    st.meta = -1;
                });
                now.set(format!("{} {}", "", &error_msg));
            }
        };
    });

    // get data. init - first 10 el. then - by input
    let search_items = move |pb: PathBuf, query: String| {
        spawn_local(async move {
            match invoke("search_items", &tauri_args!("pb": pb, "query": query)).await {
                Ok(js) => {
                    let s = from_value::<Vec<DataRecord>>(js).unwrap();
                    data.set(Some(s));
                    ref_state.update(|st| {
                        st.data = 1;
                    });
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!("{} {}", "", error_msg));
                    ref_state.update(|st| {
                        st.data = -1;
                    });
                }
            };
        });
    };

    let save_search_config = move |field: String| {
        let Some(mut meta_data) = meta.get_untracked() else {
            return;
        };
        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
        if let Some(pos) = meta_data.search_config.iter().position(|f| f == &field) {
            meta_data.search_config.remove(pos);
        } else {
            meta_data.search_config.push(field);
        }
        meta.set(Some(meta_data.clone()));

        spawn_local(async move {
            let _ = invoke(
                "save_search_config",
                &tauri_args! {
                    "pb": pb,
                    "vec": meta_data.search_config
                },
            )
            .await;
        });
    };

    let del_ref = move |name: PathBuf| {
        spawn_local(async move {
            match invoke("del_ref", &tauri_args!("val" : name.clone())).await {
                Ok(_s) => {
                    now.set(format!("{}: {:?}", tu_string!(i18n, edit.ok_del_ref), &name));
                    selected_ref.set(None::<PathBuf>);
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

    Effect::new(move |_| {
        log::info!("ref: {:?}", ref_state.get());
        log::info!("meta: {:#?}", meta.get());
        log::info!("selected_ref: {:#?}", selected_ref.get());
        //log::info!("stat: {:#?}", stat.get());
        log::info!("data: {:#?}", data.get());
    });

    // debounce for search. run if upd query_string || meta
    Effect::new(move |_| {
        let _ = query_string.get();
        let Some(_) = meta.get() else {
            return;
        };

        let pbclon = pb.clone();
        let current_query = query_string.get_untracked();

        set_timeout(
            move || {
                if query_string.get_untracked() == current_query {
                    search_items(pbclon, current_query);
                }
            },
            std::time::Duration::from_millis(250),
        );
    });
    provide_context(selected_el);
    provide_context(meta);
    view! {
        <Show when=move || { selected_el.get().is_none() } fallback=|| view! { <Ref_el /> }>
            <div class="ref">
                {move || {
                    match ref_state.get().meta {
                        -1 => {
                            view! {
                                <div class="gr">
                                    <h5 class="error">"🚫"</h5>
                                    <p class="err_send">
                                        <button on:click=move |_| del_ref(selected_ref.get().unwrap())>{t!(i18n, all.del)}</button>
                                    </p>
                                </div>
                            }
                                .into_any()
                        }
                        0 => view! { <span class="gr" aria-busy="true"></span> }.into_any(),
                        1 => {
                            let table_meta = meta.get().unwrap();
                            let sorted_search = sort_f_keys_v(table_meta.clone().search_config);
                            view! {
                                <div class="gr">
                                    <div class="header-row">
                                        <button on:click=move |_| selected_ref.set(None)>"←"</button>

                                        <div class="title-group">
                                            <span>{move || remove_refer_ext(&selected_ref.get().unwrap_or_default())}</span>
                                            <small class="meta grid">
                                                <span>{table_meta.name.clone()}</span>
                                                <span>{table_meta.desc.clone()}</span>
                                            </small>
                                        </div>

                                        <button on:click=move |_| edit_ref.set(true)>"✎"</button>
                                    </div>

                                    // input. remove if empty search_config or count = 0
                                    {
                                        let metaclon = sorted_search.clone();
                                        move || {
                                            if metaclon.is_empty() {
                                                view! {
                                                    <h5 class="warn">{t!(i18n, ref_main.no_fields_selected)}</h5>
                                                    <h6 class="warn">{t!(i18n, ref_main.fields_hint)}</h6>
                                                }
                                                    .into_any()
                                            } else {
                                                let has_data = data.get().is_some_and(|d| !d.is_empty());
                                                let query_len = query_string.get().len();
                                                log::info!("query_len:{},has_data:{}",query_len, has_data);
                                                let text = match (query_len, has_data) {
                                                    (0, true) => tu_string!(i18n, ref_main.first_records),
                                                    (0, false) => tu_string!(i18n, ref_main.ref_empty),
                                                    (_, true) => tu_string!(i18n, ref_main.found),
                                                    (_, false) => tu_string!(i18n, ref_main.nothing_found),
                                                };

                                                view! {
                                                    <input
                                                        type="text"
                                                        on:input:target=move |ev| {
                                                            query_string.set(ev.target().value());
                                                        }
                                                        prop:value=move || query_string.get()
                                                    />
                                                    <small class="m0">{text}</small>
                                                }
                                                    .into_any()
                                            }
                                        }
                                    }

                                    // search result
                                    {match sorted_search.clone().is_empty() {
                                        true => view! { <div class="search_results">""</div> }.into_any(),
                                        false => {
                                            let col_count = sorted_search.clone().len();
                                            let names = table_meta.clone().field_names;
                                            view! {
                                                <div class="search_results" style=format!("--cols: {}", col_count)>
                                                    <div class="row">
                                                        {{
                                                            f2name_v(&sorted_search, &names)
                                                                .into_iter()
                                                                .map(|n| view! { <small>{n}</small> })
                                                                .collect_view()
                                                        }}
                                                    </div>
                                                    <For
                                                        each=move || data.get().unwrap_or_default()
                                                        key=|rec: &DataRecord| rec.id
                                                        children=move |rec: DataRecord| {
                                                            let search_fields = sorted_search.clone();

                                                            view! {
                                                                <button class="row" on:click=move |_| selected_el.set(Some(rec.id))>
                                                                    {search_fields
                                                                        .into_iter()
                                                                        .filter_map(|k| { rec.fields.get(&k).map(|v| (k, v.clone())) })
                                                                        .map(|(_k, v)| {
                                                                            view! { <div>{v}</div> }
                                                                        })
                                                                        .collect_view()}
                                                                </button>
                                                            }
                                                        }
                                                    />
                                                </div>
                                            }
                                                .into_any()
                                        }
                                    }}
                                </div>

                                <div class="grid1a stat_table gr info">
                                    {
                                        let names = table_meta.clone().field_names;
                                        let oper = table_meta.clone().operations;
                                        let search_vec = table_meta.clone().search_config;
                                        view! {
                                            <b>{t!(i18n, ref_main.columns)}</b>
                                            <div class="ref_fields">
                                                {match names.is_empty() {
                                                    true => view! { <span>"-"</span> }.into_any(),
                                                    false => {
                                                        let header = // Сначала создаем заголовок
                                                        view! {
                                                            <small>
                                                                <span>{t!(i18n, ref_main.column_name)}</span>
                                                                <span>{t!(i18n, ref_main.column_type)}</span>
                                                                <span>{t!(i18n, ref_main.column_in_search)}</span>
                                                            </small>
                                                        };
                                                        let names = table_meta.field_names.clone();
                                                        let mut sorted_keys: Vec<String> = names.keys().cloned().collect();
                                                        sorted_keys
                                                            .sort_by(|a, b| {
                                                                let num_a = a[2..].parse::<i32>().unwrap_or(0);
                                                                let num_b = b[2..].parse::<i32>().unwrap_or(0);
                                                                num_a.cmp(&num_b)
                                                            });
                                                        let list = sorted_keys
                                                            .into_iter()
                                                            .filter_map(|key| {
                                                                let name = names.get(&key)?;
                                                                let ft = table_meta.field_types.get(&key)?;
                                                                Some((name.clone(), ft.clone(), key))
                                                            })
                                                            .map(|(name, ft, k)| {
                                                                view! {
                                                                    <li>
                                                                        <span>{name}</span>
                                                                        <span>{ft}</span>
                                                                        <label>
                                                                            <input
                                                                                type="checkbox"
                                                                                prop:checked={
                                                                                    let k = k.clone();
                                                                                    search_vec.contains(&k)
                                                                                }
                                                                                on:click={
                                                                                    let k = k.clone();
                                                                                    move |_| { save_search_config(k.clone()) }
                                                                                }
                                                                            />
                                                                        </label>
                                                                    </li>
                                                                }
                                                            })
                                                            .collect_view();

                                                        view! { <>{header} {list}</> }
                                                            .into_any()
                                                    }
                                                }}
                                            </div>

                                            <b>{t!(i18n, ref_main.operations)}</b>
                                            <div>
                                                {if oper.is_empty() {
                                                    view! { <span>"-"</span> }.into_any()
                                                } else {
                                                    let field_names = &table_meta.field_names;
                                                    oper.into_iter()
                                                        .map(|k| {
                                                            view! {
                                                                <div>
                                                                    <b>{k.name}</b>
                                                                    " : "
                                                                    {prettify_operation(&k.expression, field_names)}
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()
                                                        .into_any()
                                                }}
                                            </div>

                                            <b>{t!(i18n, ref_main.total_records)}</b>
                                            <span>{table_meta.count_data}</span>
                                        }
                                    }
                                </div>
                            }
                                .into_any()
                        }
                        _ => view! { <span class="gr">{t!(i18n, all.err_unknown)}</span> }.into_any(),
                    }
                }}
            </div>
        </Show>
    }
}

#[component]
pub fn Ref_el() -> impl IntoView {
    let selected_el = use_context::<RwSignal<Option<u32>>>().expect("element not found");
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let edit_el = RwSignal::new(false);
    let data = RwSignal::new(None::<DataRecord>);

    // get el
    spawn_local(async move {
        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
        match invoke(
            "get_el",
            &tauri_args!("pb": pb, "id": Some(selected_el.get_untracked())),
        )
        .await
        {
            Ok(js) => {
                let s = from_value::<DataRecord>(js).unwrap();
                data.set(Some(s))
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("{} {}", "", &error_msg));
            }
        };
    });

    Effect::new(move |_| {
        log::info!("meta: {:#?}", meta.get());
        log::info!("selected_ref: {:?}", selected_ref.get().unwrap());
        log::info!("data: {:#?}", data.get());
    });
    view! {
        <Show when=move || { !edit_el.get() } fallback=|| view! { <Ref_el_edit /> }>
            {move || {
                match data.get().is_none() {
                    true => view! { <span class="gr" aria-busy="true"></span> }.into_any(),
                    false => {
                        view! {
                            <div class="ref">
                                <div class="header-row gr">
                                    <button on:click=move |_| selected_el.set(None)>"←"</button>

                                    <div class="title-group">
                                        <div>{get_item_title(&data.get_untracked().unwrap(), &meta.get_untracked().unwrap())}</div>
                                        <small>{move || remove_refer_ext(&selected_ref.get().unwrap_or_default())}</small>
                                    </div>

                                    <button on:click=move |_| edit_el.set(true)>"🔧"</button>
                                </div>

                                <Show when=move || data.get().is_some() fallback=|| view! { <div>"No data"</div> }>
                                    <div class="grid1a gr">
                                        {move || {
                                            let d = data.get().unwrap();
                                            let m = meta.get().unwrap();
                                            let mut items: Vec<_> = d.fields.iter().collect();
                                            items.sort_by_key(|(k, _)| *k);
                                            items
                                                .into_iter()
                                                .map(|(k, v)| {
                                                    let display_name = m
                                                        .field_names
                                                        .get(k)
                                                        .map(|n| n.to_string())
                                                        .unwrap_or_else(|| k.to_string());
                                                    view! {
                                                        <strong>{display_name}</strong>
                                                        <div>{v.to_string()}</div>
                                                    }
                                                })
                                                .collect_view()
                                        }}
                                    </div>
                                </Show>

                                <Show
                                    when=move || !meta.get().unwrap().operations.is_empty()
                                    fallback=|| {
                                        view! {
                                            <div class="gr">
                                                <h5>"No saved operations"</h5>
                                            </div>
                                        }
                                    }
                                >
                                    {
                                        let metaclon = meta.get().unwrap();
                                        let dataclon = data.get().unwrap().fields;
                                        let (inputs, calcs): (Vec<_>, Vec<_>) = meta
                                            .get()
                                            .unwrap()
                                            .operations
                                            .into_iter()
                                            .partition(|op| op.expression.contains("input_"));
                                        let calcs_view = calcs
                                            .into_iter()
                                            .map(|op| {
                                                let oper = op.expression.clone();
                                                let res = run_oper(&oper, &metaclon, &dataclon);
                                                log::debug!("Результат: {:?}", res);

                                                view! {
                                                    <div class="oper gr grid">
                                                        <div class="gridl">
                                                            <strong>{op.name}</strong>
                                                            <span>
                                                                {match res {
                                                                    Ok(val) => view! { <span class="success">{val}</span> }.into_view(),
                                                                    Err(e) => view! { <span class="error">{e}</span> }.into_view(),
                                                                }}
                                                            </span>
                                                        </div>
                                                        <span>{op.description}</span>
                                                        <span>{prettify_operation(&oper, &metaclon.field_names)}</span>
                                                    </div>
                                                }
                                            })
                                            .collect_view();
                                        let inputs_view = inputs
                                            .into_iter()
                                            .map(|op| {
                                                let oper = op.expression.clone();

                                                // let res = run_oper(&oper, &metaclon, &dataclon);

                                                view! {
                                                    <div class="oper gr">
                                                        <div class="gridl">
                                                            <strong>{op.name}</strong>
                                                            <span></span>
                                                        </div>
                                                        <span>{op.description}</span>
                                                        <span>{prettify_operation(&oper, &metaclon.field_names)}</span>
                                                    </div>
                                                }
                                            })
                                            .collect_view();
                                        view! {
                                            {calcs_view}
                                            {inputs_view}
                                        }
                                    }
                                </Show>
                            </div>
                        }
                            .into_any()
                    }
                }
            }}
        </Show>
    }
}

#[component]
fn Ref_el_edit() -> impl IntoView {
   
    view! { "Ref_el_edit" }
}

fn check_oper(oper: &String) { 
    //let result = Vec::new();
    //let mut ff = [];
    //let mut inputs = [];
    //let tokens: Vec<&str> = oper.split_whitespace().collect();
}

fn run_oper(operation: &str, meta: &TableMeta, data: &HashMap<String, String>) -> Result<String, String> {
    let expr = exmex::parse::<f64>(operation).map_err(|e| e.to_string()).unwrap();

    let mut var_values = Vec::new();
    for var_name in expr.var_names() {
        let val_str = data.get(var_name).ok_or("Variable missing")?;
        let val_num: f64 = val_str.parse().map_err(|_| "Parse error")?;
        var_values.push(val_num);
    }
    let result = expr.eval(&var_values).map_err(|e| e.to_string());

    match result {
        Ok(result) => Ok(result.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn prettify_operation(
    expression: &str,
    field_map: &HashMap<String, String>, // {"f_2": "Velocity (m/s)"}
) -> String {
    let expr = match parse::<f64>(expression) {
        Ok(e) => e,
        Err(_) => return expression.to_string(), // Если не парсится — возвращаем как есть
    };

    let mut result = expression.to_string();

    // exmex сам найдет все переменные в правильном порядке
    let vars = expr.var_names();

    // Сортируем по длине (убывание) для корректной замены
    let mut sorted_vars: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    sorted_vars.sort_by_key(|b| std::cmp::Reverse(b.len()));

    for var in sorted_vars {
        if var.starts_with("f_") {
            // Полевая переменная → красивое имя
            if let Some(display) = field_map.get(var) {
                result = result.replace(var, display);
            }
        } else if var.starts_with("input_") {
            // Инпут → [плейсхолдер]
            let placeholder = var.strip_prefix("input_").unwrap();
            result = result.replace(var, placeholder);
        }
    }

    result
}

// let result = expr.eval(&[3.7, 2.5, 1.0]).map_err(|e| e.to_string());
//expr.eval(&[3.7, 2.5, 1.0])   .map(|v| v.to_string())
//run_oper("α * ln(z) + 2* (-z^2 + sin(4*y))".to_string(),vec![3.7, 2.5, 1.0]);
/*
"f_1 * f_2 * f_2 / 2000",
"f_1 / (f_4 * 1000)",
"((9.81 * (distance / f_2) * (distance / f_2)) / 2) * 100",
"wind_speed * (distance / f_2) * (1 / f_3) * 100"
*/
