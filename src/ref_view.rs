use crate::{app::*, functions::*, i18n::*, ref_edit::*, tauri_args};
use core::f64;
use exmex::prelude::*;
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
        //log::info!("ref: {:?}", ref_state.get());
        log::info!("meta: {:#?}", meta.get());
        //log::info!("selected_ref: {:#?}", selected_ref.get());
        //log::info!("stat: {:#?}", stat.get());
        //log::info!("data: {:#?}", data.get());
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

                                    // input. dont show if empty search_config or count < 11
                                    {
                                        let sorted_search_clone = sorted_search.clone();
                                        move || {
                                            if sorted_search_clone.is_empty() {
                                                // log::info!("count_data:{:?}",&table_meta.count_data );
                                                view! {
                                                    <h5 class="warn">{t!(i18n, ref_main.no_fields_selected)}</h5>
                                                    <h6 class="warn">{t!(i18n, ref_main.fields_hint)}</h6>
                                                }
                                                    .into_any()
                                            } else if table_meta.count_data < 11 {
                                                view! { "" }.into_any()
                                            } else {
                                                let has_data = data.get().is_some_and(|d| !d.is_empty());
                                                let query_len = query_string.get().len();
                                                let text = match (query_len, has_data) {
                                                    (0, true) => tu_string!(i18n, ref_main.first_records),
                                                    (0, false) => tu_string!(i18n, ref_main.ref_empty),
                                                    (_, true) => tu_string!(i18n, ref_main.found),
                                                    (_, false) => tu_string!(i18n, ref_main.nothing_found),
                                                };
                                                // log::info!("query_len:{},has_data:{}",query_len, has_data);

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
                                            <div class="grid">
                                                {if oper.is_empty() {
                                                    view! { <span>"-"</span> }.into_any()
                                                } else {
                                                    let field_names = &table_meta.field_names;
                                                    oper.into_iter()
                                                        .map(|k| {
                                                            view! {
                                                                <div class="grid">
                                                                    <span class="gridline">
                                                                        {k.name}" "
                                                                        {(!k.description.is_empty())
                                                                            .then(|| format!("({})", k.description))}
                                                                    </span>
                                                                    <small>{prettify_operation(&k.expression, field_names)}</small>
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
        //log::info!("selected_ref: {:?}", selected_ref.get().unwrap());
        //log::info!("data: {:#?}", data.get());
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
                                    <div class="grid2 gr">
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
                                                        <span>{display_name}</span>
                                                        <span>{v.to_string()}</span>
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
                                        let (inputs, calcs): (Vec<_>, Vec<_>) = meta
                                            .get()
                                            .unwrap()
                                            .operations
                                            .into_iter()
                                            .partition(|op| op.expression.contains("input_"));
                                        // let metaclon = meta.get().unwrap();
                                        // let dataclon = data.get().unwrap().fields;

                                        view! {
                                            <For
                                                each=move || calcs.clone()
                                                key=|op| op.id
                                                children=move |op| {
                                                    let oper = op.expression.clone();
                                                    let res = run_oper(&oper, &data.get().unwrap().fields);
                                                    let op_id = op.id;
                                                    view! {
                                                        <div class="oper gr grid center">
                                                            <strong class="center">
                                                                {op.name}" : "
                                                                <span>
                                                                    {match res {
                                                                        Ok(val) => {
                                                                            view! {
                                                                                <span class="success">
                                                                                    {format!("{:.*}", op.precision as usize, val)}
                                                                                </span>
                                                                            }
                                                                                .into_any()
                                                                        }
                                                                        Err(e) => view! { <span class="error">{e}</span> }.into_any(),
                                                                    }}
                                                                </span>"  " <Prec op_id=op_id />
                                                            </strong>
                                                            <small>{op.description}</small>
                                                            <small>{prettify_operation(&oper, &meta.get().unwrap().field_names)}</small>
                                                        </div>
                                                    }
                                                }
                                            />

                                            <For
                                                each=move || inputs.clone()
                                                key=|op| op.id
                                                children=move |op| {
                                                    view! { <RunOperIn op_id=op.id data=data.get().unwrap().fields /> }
                                                }
                                            />
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
fn RunOperIn(op_id: u32, data: HashMap<String, String>) -> impl IntoView {
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");

    let (initial_prec, op_name, op_expr_str, op_desc, names) = {
        let m = meta.get_untracked().expect("meta empty");
        let o = m.operations.iter().find(|o| o.id == op_id).expect("op not found");
        (
            o.precision,
            o.name.clone(),
            o.expression.clone(),
            o.description.clone(),
            m.field_names.clone(),
        )
    };

    let (local_precision, set_local_precision) = signal(initial_prec);
    let (input_values, set_input_values) = signal(HashMap::<String, String>::new());
    let (result, set_result) = signal(None::<f64>);

    let input_names: Vec<String> = {
        let expr = exmex::parse::<f64>(&op_expr_str).expect("failed to parse");
        expr.var_names()
            .iter()
            .filter(|n| n.starts_with("input_"))
            .cloned()
            .collect()
    };

    let is_all_filled = Memo::new({
        let names = input_names.clone();
        move |_| {
            input_values
                .with(|map| !names.is_empty() && names.iter().all(|n| map.get(n).is_some_and(|v| !v.is_empty())))
        }
    });

    let calc_expr = op_expr_str.clone();
    let calc_data = data.clone();

    let on_calc = move |_| {
        if !is_all_filled.get() {
            set_result.set(None);
            return;
        }
        let mut all_data = calc_data.clone();
        input_values.with(|map| {
            for (k, v) in map {
                if !v.is_empty() {
                    all_data.insert(k.clone(), v.clone());
                }
            }
        });
        if let Ok(val) = run_oper(&calc_expr, &all_data) {
            set_result.set(Some(val));
        }
    };

    let on_clear = move |_| {
        set_input_values.set(HashMap::new());
        set_result.set(None);
    };

    let set_prec = move |ev: web_sys::Event| {
        let new_val = event_target_value(&ev).parse().unwrap_or(2);
        set_local_precision.set(new_val);

        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
        let mut old_operations = meta.get_untracked().unwrap().operations;
        if let Some(op) = old_operations.iter_mut().find(|op| op.id == op_id) {
            op.precision = new_val;
        }
        //let new_operations = old_operations.clone();

        spawn_local(async move {
            let _ = invoke(
                "update_meta_field",
                &tauri_args! {
                    "pb": pb,
                    "key": "operations",
                    "value": old_operations
                },
            )
            .await;
        });
    };

    view! {
        <div class="oper gr grid center">
            <strong class="center">
                {op_name} " : "
                <span class="success">
                    {move || {
                        let p = local_precision.get() as usize;
                        match result.get() {
                            Some(val) => format!("{:.*}", p, val),
                            None => "?".to_string(),
                        }
                    }}
                </span>"  " <select class="precision" on:change=set_prec prop:value=move || local_precision.get()>
                    {(0..18).map(|n| view! { <option value=n>{n}</option> }).collect_view()}
                </select>
            </strong>

            <div class="flex_wrap3">
                {input_names
                    .into_iter()
                    .map(|name| {
                        let n1 = name.clone();
                        let n2 = name.clone();
                        let n3 = name.clone();
                        let display = name.strip_prefix("input_").unwrap_or(&name).to_string();
                        view! {
                            <label>
                                <small>{display}</small>
                                <input
                                    type="text"
                                    inputmode="decimal"
                                    prop:value=move || input_values.with(|m| m.get(&n1).cloned().unwrap_or_default())
                                    on:input=move |ev| {
                                        let node = event_target::<web_sys::HtmlInputElement>(&ev);
                                        let raw = node.value().replace(',', ".");
                                        let is_valid = raw
                                            .chars()
                                            .enumerate()
                                            .all(|(i, c)| { c.is_ascii_digit() || c == '.' || (c == '-' && i == 0) })
                                            && raw.matches('.').count() <= 1 && raw.matches('-').count() <= 1;
                                        if is_valid || raw.is_empty() {
                                            let n = n2.clone();
                                            set_input_values
                                                .update(|m| {
                                                    m.insert(n, raw);
                                                });
                                        } else {
                                            node.set_value(&input_values.get_untracked().get(&n3).cloned().unwrap_or_default());
                                        }
                                    }
                                />
                            </label>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="flex_wrap3 center">
                <button on:click=on_clear>"🧹"</button>
                <button on:click=on_calc prop:disabled=move || !is_all_filled.get()>
                    "="
                </button>
            </div>

            <small>{op_desc}</small>
            <small>{prettify_operation(&op_expr_str, &names)}</small>
        </div>
    }
}

#[component]
fn Prec(op_id: u32) -> impl IntoView {
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");

    let precision = move || {
        meta.get()
            .unwrap()
            .operations
            .iter()
            .find(|o| o.id == op_id)
            .unwrap()
            .precision
    };

    let on_input = move |ev: web_sys::Event| {
        let new_val = event_target_value(&ev).parse().unwrap_or(2);
        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());

        meta.update(|meta_opt| {
            let meta = meta_opt.as_mut().unwrap();
            let operation = meta.operations.iter_mut().find(|o| o.id == op_id).unwrap();
            operation.precision = new_val;
        });

        let for_send = meta.get_untracked().unwrap().operations;

        spawn_local(async move {
            let _ = invoke(
                "update_meta_field",
                &tauri_args! {
                    "pb": pb,
                    "key": "operations",
                    "value": for_send
                },
            )
            .await;
        });
    };

    view! {
        <select class="precision" on:change=on_input prop:value=precision>
            {(0..18).map(|n| view! { <option value=n>{n}</option> }).collect_view()}
        </select>
    }
}

fn run_oper(operation: &str, data: &HashMap<String, String>) -> Result<f64, String> {
    let expr = exmex::parse::<f64>(operation).map_err(|e| e.to_string()).unwrap();

    let mut var_values = Vec::new();
    for var_name in expr.var_names() {
        let val_str = data.get(var_name).ok_or("Variable missing")?;
        let val_num: f64 = val_str.parse().map_err(|_| "Parse error")?;
        var_values.push(val_num);
    }
    let result = expr.eval(&var_values).map_err(|e| e.to_string());

    match result {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}

pub fn prettify_operation(
    expression: &str,
    field_map: &HashMap<String, String>, // {"f_2": "Velocity (m/s)"}
) -> String {
    use exmex::{parse, prelude::*};
    let expr = match parse::<f64>(expression) {
        Ok(e) => e,
        Err(_) => return expression.to_string(), // Если не парсится — возвращаем как есть
    };

    let mut result = expression.to_string();

    let vars = expr.var_names();

    // Сортируем по длине (убывание) для корректной замены
    let mut sorted_vars: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    sorted_vars.sort_by_key(|b| std::cmp::Reverse(b.len()));

    for var in sorted_vars {
        if var.starts_with("f_") {
            if let Some(display) = field_map.get(var) {
                result = result.replace(var, display);
            }
        } else if var.starts_with("input_") {
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
