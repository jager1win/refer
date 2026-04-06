use crate::{app::*, functions::*, i18n::*, ref_edit::*, tauri_args};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
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
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("reference not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
    let query_string = RwSignal::new("".to_string());
    let meta = RwSignal::new(None::<TableMeta>);
    let data = RwSignal::new(None::<Vec<DataRecord>>);
    let ref_state = RwSignal::new(RefState::new());

    // get meta
    spawn_local(async move {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
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

    // get data. всегда по 10 элементов.
    let search_items = move |pb: PathBuf, query: String| {
        spawn_local(async move {
            match invoke("search_items", &tauri_args!("pb": pb, "query": query)).await {
                Ok(js) => {
                    let s = from_value::<Vec<DataRecord>>(js).unwrap();
                    ref_state.update(|st| {
                        if query.is_empty() && s.is_empty() {
                            st.data = -1;
                        } else {
                            st.data = 1;
                            data.set(Some(s));
                        }
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

    Effect::new(move |_| {
        //log::info!("ref: {:?}", ref_state.get());
        //log::info!("meta: {:#?}", meta.get());
        //log::info!("selected: {:#?}", selected.get());
        //log::info!("stat: {:#?}", stat.get());
        //log::info!("data: {:#?}", data.get());
    });

    // debounce for search. run if upd query_string || meta
    Effect::new(move |_| {
        let _ = query_string.get();
        let Some(_) = meta.get() else {
            return;
        };
        if meta.get().unwrap().count_data == 0 {
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
    provide_context(meta);
    view! {
        <Show when=move || { selected.get().element.is_none() } fallback=|| view! { <Ref_el /> }>
            <div class="ref">
                {move || {
                    match ref_state.get().meta {
                        -1 => {
                            view! {
                                <div class="gr">
                                    <h5 class="error">"🚫"</h5>
                                    <p class="err_send">
                                        <button on:click=move |_| del_ref(selected.get().refer.unwrap())>{t!(i18n, all.del)}</button>
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
                                <div class="header-row gr">
                                    <button on:click=move |_| selected.update(|c| c.refer = None)>"←"</button>

                                    <div class="title-group">
                                        <span>{move || remove_refer_ext(&selected.get().refer.unwrap())}</span>
                                        <small class="meta grid">
                                            <span>{table_meta.name.clone()}</span>
                                            <span>{table_meta.desc.clone()}</span>
                                        </small>
                                    </div>

                                    <button on:click=move |_| edit_ref.set(true)>"✎"</button>
                                </div>

                                <div class="gr">
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
                                    } // search result
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
                                                                <button
                                                                    class="row"
                                                                    on:click=move |_| selected.update(|c| c.element = Some(rec.id))
                                                                >
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

                                <div class="grid1a stat_table gr info a-start">
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
                                                    oper.into_iter()
                                                        .map(|k| {
                                                            view! {
                                                                <div class="grid">
                                                                    <span class="gridline">
                                                                        {k.name}" "
                                                                        {(!k.description.is_empty())
                                                                            .then(|| format!("({})", k.description))}
                                                                    </span>
                                                                    <small>{k.expression}</small>
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
    let i18n = use_i18n();
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
    let settings = use_context::<RwSignal<AppSettings>>().expect("settings not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let edit_el = RwSignal::new(false);
    let data = RwSignal::new(None::<DataRecord>);

    let in_qa = move || {
        let (refer, element) = (selected.get().refer.clone().unwrap(), selected.get().element.unwrap());
        settings.with(|s| s.qa.iter().any(|item| item.path == refer && item.id == element))
    };

    // get el
    spawn_local(async move {
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
        match invoke(
            "get_el",
            &tauri_args!("pb": pb, "id": Some(selected.get_untracked().element)),
        )
        .await
        {
            Ok(js) => {
                let s = from_value::<DataRecord>(js).unwrap();
                data.set(Some(s))
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("{} {}", "Error:", &error_msg));
            }
        };
    });

    let toggle_qa = move |title| {
        let (refer, element) = (selected.get().refer.clone().unwrap(), selected.get().element.unwrap());

        settings.update(|s| {
            if let Some(pos) = s.qa.iter().position(|item| item.path == refer && item.id == element) {
                s.qa.remove(pos);
                now.set(format!("- {}", tu_string!(i18n, all.qa)));
            } else {
                s.qa.push(QuickAccess {
                    path: refer,
                    id: element,
                    name: title,
                });
                now.set(format!("+ {}", tu_string!(i18n, all.qa)));
            }
        });

        spawn_local(async move {
            let _ = invoke("set_settings", &tauri_args!("new": settings.get_untracked())).await;
        });
    };

    let remove_qa = move || {
        let remove = selected.get_untracked();
        settings.update(|s| {
            if let Some(pos) = s.qa.iter().position(|item| &item.path == remove.refer.as_ref().unwrap() && item.id == remove.element.unwrap()) {
                s.qa.remove(pos);
                now.set(format!("- {}", tu_string!(i18n, all.qa)));
                selected.update(|c| {c.refer = None;c.element = None;});
            }
        });
        spawn_local(async move {
            let _ = invoke("set_settings", &tauri_args!("new": settings.get_untracked())).await;
        });
    };

    Effect::new(move |_| {
        //log::info!("meta: {:#?}", meta.get());
        //log::info!("selected {:?}", selected.get());
        //log::info!("data: {:#?}", data.get());
        //log::info!("sett: {:#?}", settings.get());
    });
    view! {
        <Show when=move || { !edit_el.get() } fallback=|| view! { <Ref_el_edit /> }>
            {move || {
                match data.get().is_none() {
                    true => {
                        view! {
                            <div class="gr">
                                <h5 class="error">"🚫"</h5>
                                <p class="err_send">
                                    <span class="">{t!(i18n, all.element_not_found)}</span>
                                    {move || {
                                        if in_qa() {
                                            view! { <button class="ml1" on:click=move |_| remove_qa()>"🗑️"</button> }
                                                .into_any()
                                        } else {
                                            view! { <span>{t!(i18n, all.err_unknown)}</span> }.into_any()
                                        }
                                    }}
                                </p>
                            </div>
                        }
                            .into_any()
                    }
                    false => {
                        let title = get_item_title(&data.get_untracked().unwrap(), &meta.get_untracked().unwrap());
                        view! {
                            <div class="ref">
                                <div class="header-row gr">
                                    <button on:click=move |_| selected.update(|c| c.element = None)>"←"</button>

                                    <div class="title-group">
                                        <div>
                                            <span>{title.clone()}</span>
                                            <button
                                                class="ml1 sm_b"
                                                on:click=move |_| toggle_qa(
                                                    format!(
                                                        "{} | {}",
                                                        remove_refer_ext(&selected.get_untracked().refer.unwrap()),
                                                        title.clone(),
                                                    ),
                                                )
                                            >
                                                {move || if in_qa() { "📍" } else { "📌" }}
                                            </button>
                                        </div>
                                        <small>{move || remove_refer_ext(&selected.get().refer.unwrap())}</small>
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
                                    {move || {
                                        match transform_fields(
                                            &meta.get().unwrap().field_names,
                                            &meta.get().unwrap().field_types,
                                            &data.get().unwrap().fields,
                                        ) {
                                            Err(e) => view! { <p>{e}</p> }.into_any(),
                                            Ok(vars) => {
                                                meta.get()
                                                    .unwrap()
                                                    .operations
                                                    .into_iter()
                                                    .map(|op| { view! { <RunOper oper=op vars=vars.clone() /> }.into_any() })
                                                    .collect_view()
                                                    .into_any()
                                            }
                                        }
                                    }}
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

/*#[component]
fn RunOperIn(op_id: u32, data: HashMap<String, String>) -> impl IntoView {
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
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

        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());
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
}*/
