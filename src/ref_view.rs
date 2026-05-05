use crate::{app::*, i18n::*, ref_edit::*, state::State, tauri_args};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)] // -1,0,1
pub struct RefState {
    pub meta: i8,
    pub data: i8,
}

#[component]
pub fn Ref_main() -> impl IntoView {
    let i18n = use_i18n();
    let st = use_context::<State>().expect("State missing");
    let query_string = RwSignal::new("".to_string());
    let data = RwSignal::new(None::<Vec<DataRecord>>);
    let ref_state = RwSignal::new(RefState { meta: 0, data: 0 });
    let search_run = RwSignal::new(false);

    // get meta
    spawn_local(async move {
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        match invoke("get_meta", &tauri_args!("pb": pb)).await {
            Ok(js) => {
                let s = from_value::<TableMeta>(js).unwrap();
                ref_state.update(|rst| {
                    rst.meta = 1;
                });
                st.meta.set(Some(s))
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                ref_state.update(|rst| {
                    rst.meta = -1;
                });
                st.now.set(format!("{} {}", "", &error_msg));
            }
        };
    });

    // get data. всегда по 10 элементов.
    let search_items = move |pb: PathBuf, query: String| {
        spawn_local(async move {
            search_run.set(true);
            match invoke("search_items", &tauri_args!("pb": pb, "query": query)).await {
                Ok(js) => {
                    let s = from_value::<Vec<DataRecord>>(js).unwrap();
                    ref_state.update(|rst| {
                        if query.is_empty() && s.is_empty() {
                            rst.data = -1;
                            data.set(None);
                        } else {
                            rst.data = 1;
                            data.set(Some(s));
                        }
                    });
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    st.now.set(format!("{} {}", "", error_msg));
                    ref_state.update(|rst| {
                        rst.data = -1;
                    });
                }
            };
            search_run.set(false);
        });
    };

    let save_search_config = move |field: String| {
        let Some(mut meta_data) = st.meta.get_untracked() else {
            return;
        };
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        if let Some(pos) = meta_data.search_config.iter().position(|f| f == &field) {
            meta_data.search_config.remove(pos);
        } else {
            meta_data.search_config.push(field);
        }
        st.meta.set(Some(meta_data.clone()));

        spawn_local(async move {
            match invoke(
                "update_meta_entity",
                &tauri_args!("pb": pb,"key": "search_config","value": meta_data.search_config),
            )
            .await
            {
                Ok(_s) => {
                    st.now.set(format!("{:?}", tu_string!(i18n, edit.saved)));
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    st.now.set(format!("Error: {:?}", error_msg));
                }
            }
        });
    };

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

    // first run
    Effect::new(move |_| {
        st.selected.track();
        log::info!("first run return");
        if st.meta.get().is_none() {
            log::info!("return");
            return;
        }

        let met = st.meta.get().unwrap();
        if met.search_config.is_empty() || met.count_data == 0 {
            data.set(None);
            ref_state.update(|st| st.data = if met.count_data == 0 { -1 } else { 0 });
            return;
        }

        let pbclon = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let current_query = query_string.get_untracked();

        // Если query_string пустой — первый вход, показываем первые 10
        // Если не пустой — вернулись из элемента, восстанавливаем поиск
        search_items(pbclon, current_query);
    });

    // debounce for search. run if upd query_string || meta
    Effect::new(move |_| {
        let _ = query_string.get();
        let Some(_) = st.meta.get() else {
            return;
        };
        if st.meta.get().unwrap().count_data == 0 {
            return;
        };

        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
        let current_query = query_string.get_untracked();

        set_timeout(
            move || {
                if query_string.get_untracked() == current_query {
                    // Откладываем тяжёлую работу
                    request_animation_frame(move || {
                        search_items(pb, current_query);
                    });
                }
            },
            std::time::Duration::from_millis(150),
        );
    });

    Effect::new(move |_| {
        log::info!("ref_state: {:?}", ref_state.get());
        //log::info!("meta: {:#?}", meta.get());
        //log::info!("selected: {:#?}", selected.get());
        //log::info!("stat: {:#?}", stat.get());
        //log::info!("ref_main data: {:#?}", data.get());
    });

    view! {
        <Show when=move || { st.selected.get().id.is_none() } fallback=|| view! { <Ref_el /> }>
            <div class="ref">
                {move || {
                    match ref_state.get().meta {
                        -1 => {
                            view! {
                                <div class="gr">
                                    <h5 class="error">"🚫"</h5>
                                    <p class="err_send">
                                        <button on:click=move |_| del_ref(st.selected.get().refer.unwrap())>{t!(i18n, all.del)}</button>
                                    </p>
                                </div>
                            }
                                .into_any()
                        }
                        0 => view! { <p class="gr center" aria-busy="true"></p> }.into_any(),
                        1 => {
                            let table_meta = st.meta.get().unwrap();
                            let sorted_search: Vec<String> = table_meta
                                .names
                                .iter()
                                .filter(|name| table_meta.search_config.contains(*name))
                                .filter_map(|name| table_meta.fields.get(name))
                                .map(|field_def| field_def.name.clone())
                                .collect();
                            view! {
                                <div class="header-row gr">
                                    <button on:click=move |_| st.selected.update(|c| c.refer = None)>"←"</button>

                                    <div class="title-group">
                                        <span>{move || st.selected.get().refer.unwrap().display().to_string()}</span>
                                        // name & desc
                                        <p class="info-show m0">
                                            {match table_meta.info[0].1.is_empty() && table_meta.info[1].1.is_empty() {
                                                true => view! { <span></span> }.into_any(),
                                                false => {
                                                    view! {
                                                        {table_meta
                                                            .info
                                                            .clone()
                                                            .into_iter()
                                                            .map(|(_k, v)| view! { <span>{v}</span> })
                                                            .collect_view()}
                                                    }
                                                        .into_any()
                                                }
                                            }}
                                        </p>
                                    </div>

                                    <button on:click=move |_| st.edit_ref.set(true)>"🔧"</button>
                                </div>

                                // input. add "hidden" if empty search_config or count < 11
                                <div class="gr">
                                    <div class="search-input">
                                        {
                                            let tm_clon = table_meta.clone();
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
                                                <div class:hidden=move || {
                                                    (tm_clon.count_data < 11) || (tm_clon.search_config.is_empty())
                                                }>
                                                    <div class="header-row">
                                                        <label for="peas">
                                                            {move || match search_run.get() {
                                                                true => view! { <span aria-busy="true"></span> }.into_any(),
                                                                false => view! { <span>"🔍"</span> }.into_any(),
                                                            }}
                                                        </label>
                                                        <input
                                                            type="text"
                                                            name="peas"
                                                            on:input:target=move |ev| {
                                                                query_string.set(ev.target().value());
                                                            }
                                                            prop:value=move || query_string.get()
                                                        />

                                                        <span class="">{text}</span>
                                                    </div>
                                                    <hr />
                                                </div>
                                            }
                                        }
                                        {(table_meta.search_config.is_empty())
                                            .then(|| {
                                                view! {
                                                    <h5 class="warn">{t!(i18n, ref_main.no_fields_selected)}</h5>
                                                    <h6 class="warn">{t!(i18n, ref_main.fields_hint)}</h6>
                                                }
                                            })}
                                    </div>

                                    // search result
                                    {match data.get() {
                                        None => view! { <div class="search_results"></div> }.into_any(),
                                        Some(d) => {
                                            let col_count = sorted_search.clone().len();
                                            let meta = table_meta.clone();
                                            let names = sorted_search.clone();
                                            view! {
                                                <div class="search_results" style=format!("--cols: {}", col_count + 1)>
                                                    <div class="row">
                                                        <small>ID</small>
                                                        {{ names.into_iter().map(|n| view! { <small>{n}</small> }).collect_view() }}
                                                    </div>
                                                    <For
                                                        each=move || d.clone()
                                                        key=|rec: &DataRecord| rec.id
                                                        children=move |rec: DataRecord| {
                                                            let search_fields_info = meta.search_config.clone();

                                                            view! {
                                                                <button
                                                                    class="row"
                                                                    on:click=move |_| st.selected.update(|c| c.id = Some(rec.id))
                                                                >
                                                                    <div>{format!("#{}", rec.id)}</div>
                                                                    {search_fields_info
                                                                        .iter()
                                                                        .filter_map(|k| rec.fields.get(k))
                                                                        .map(|v| view! { <div>{v.clone()}</div> })
                                                                        .collect::<Vec<_>>()}
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
                                        let oper = table_meta.clone().operations;
                                        let search_vec = table_meta.clone().search_config;
                                        let full_pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());

                                        view! {
                                            <b>{t!(i18n, ref_main.total_records)}</b>
                                            <span>{table_meta.count_data}</span>

                                            <b>{t!(i18n, ref_main.columns)}</b>
                                            <div class="ref_fields">
                                                {match table_meta.names.is_empty() {
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
                                                        let list = table_meta
                                                            .names
                                                            .iter()
                                                            .map(|field_key| {
                                                                let field_def = table_meta.fields.get(field_key).unwrap();
                                                                view! {
                                                                    <li>
                                                                        <span>{field_def.name.clone()}</span>
                                                                        <span>{field_def.ftype.clone()}</span>
                                                                        <label>
                                                                            <input
                                                                                type="checkbox"
                                                                                prop:checked=search_vec.contains(field_key)
                                                                                on:click={
                                                                                    let k = field_key.clone();
                                                                                    move |_| save_search_config(k.clone())
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
                                                                <div class="grid gap02">
                                                                    <span class="gridline">
                                                                        {k.name}" "
                                                                        {(!k.desc.is_empty()).then(|| format!(" • {}", k.desc))}
                                                                        " • "{t!(i18n, ref_main.prec)}": "{k.prec}
                                                                    </span>
                                                                    <small>{k.expr}</small>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()
                                                        .into_any()
                                                }}
                                            </div>

                                            <b>{t!(i18n, refs.st_path)}</b>
                                            <span>{full_pb.display().to_string()}</span>
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
    let st = use_context::<State>().expect("State missing");
    let edit_el = RwSignal::new(false);
    let data = RwSignal::new(None::<DataRecord>);
    let ref_state = RwSignal::new(RefState { meta: 1, data: 0 });

    let in_qa = move || {
        let (refer, element) = (st.selected.get().refer.clone().unwrap(), st.selected.get().id.unwrap());
        st.settings
            .with(|s| s.qa.iter().any(|item| item.path == refer && item.id == element))
    };

    // get el
    let get_el = move || {
        spawn_local(async move {
            let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());
            match invoke("get_el", &tauri_args!("pb": pb, "id": Some(st.selected.get_untracked().id))).await {
                Ok(js) => {
                    let s = from_value::<DataRecord>(js).unwrap();
                    data.set(Some(s));
                    ref_state.update(|rst| {
                        rst.data = 1;
                    });
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    st.now.set(format!("{} {}", "Error:", &error_msg));
                    ref_state.update(|rst| {
                        rst.data = -1;
                    });
                }
            };
        })
    };
    get_el();

    let toggle_qa = move |title| {
        let (refer, element) = (st.selected.get().refer.clone().unwrap(), st.selected.get().id.unwrap());

        st.settings.update(|s| {
            if let Some(pos) = s.qa.iter().position(|item| item.path == refer && item.id == element) {
                s.qa.remove(pos);
                st.now.set(format!("- {}", tu_string!(i18n, all.qa)));
            } else {
                s.qa.push(QuickAccess {
                    path: refer,
                    id: element,
                    name: title,
                });
                st.now.set(format!("+ {}", tu_string!(i18n, all.qa)));
            }
        });

        spawn_local(async move {
            let _ = invoke("set_settings", &tauri_args!("new": st.settings.get_untracked())).await;
        });
    };

    let remove_qa = move || {
        let remove = st.selected.get_untracked();
        st.settings.update(|s| {
            if let Some(pos) =
                s.qa.iter()
                    .position(|item| &item.path == remove.refer.as_ref().unwrap() && item.id == remove.id.unwrap())
            {
                s.qa.remove(pos);
                st.now.set(format!("- {}", tu_string!(i18n, all.qa)));
                st.selected.update(|c| {
                    c.refer = None;
                    c.id = None;
                });
            }
        });
        spawn_local(async move {
            let _ = invoke("set_settings", &tauri_args!("new": st.settings.get_untracked())).await;
        });
    };

    Effect::new(move |_| {
        edit_el.track();
        get_el();
    });

    Effect::new(move |_| {
        //log::info!("meta: {:#?}", meta.get());
        //log::info!("selected {:?}", selected.get());
        //log::info!("ref_el data: {:#?}", data.get());
        //log::info!("sett: {:#?}", settings.get());
    });
    provide_context(edit_el);
    view! {
        <Show when=move || { !edit_el.get() } fallback=move || view! { <Ref_el_edit /> }>
            {move || {
                match ref_state.get().data {
                    -1 => {
                        view! {
                            <div class="gr">
                                <h5 class="error">"🚫"</h5>
                                <p class="err_send">
                                    <span class="">{t!(i18n, all.element_not_found)}</span>
                                    {move || {
                                        if in_qa() {
                                            view! {
                                                <button class="ml1" on:click=move |_| remove_qa()>
                                                    "🗑️"
                                                </button>
                                            }
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
                    0 => view! { <p class="gr center" aria-busy="true"></p> }.into_any(),
                    1 => {
                        let title = get_item_title(&data.get_untracked().unwrap(), &st.meta.get_untracked().unwrap());
                        view! {
                            <div class="ref">
                                <div class="header-row gr">
                                    <button on:click=move |_| {
                                        st.load_meta();
                                        st.selected.update(|c| c.id = None)
                                    }>"←"</button>

                                    <div class="title-group">
                                        <div>
                                            <span>{title.clone()}</span>
                                            <button
                                                class="ml1 sm_b"
                                                on:click=move |_| toggle_qa(
                                                    format!(
                                                        "{} | {}",
                                                        st.remove_refer_ext(&st.selected.get_untracked().refer.unwrap()),
                                                        title.clone(),
                                                    ),
                                                )
                                            >
                                                {move || if in_qa() { "📍" } else { "📌" }}
                                            </button>
                                        </div>
                                        <small>{move || st.selected.get_untracked().refer.unwrap().display().to_string()}</small>
                                    </div>

                                    <button on:click=move |_| edit_el.set(true)>"🔧"</button>
                                </div>

                                <Show when=move || data.get().is_some() fallback=|| view! { <div>"No data"</div> }>
                                    <div class="grid2 gr">
                                        {move || {
                                            let d = data.get().unwrap();
                                            let m = st.meta.get().unwrap();
                                            let mut items: Vec<_> = d.fields.iter().collect();
                                            items.sort_by_key(|(k, _)| *k);
                                            items
                                                .into_iter()
                                                .map(|(k, v)| {
                                                    let display_name = m
                                                        .fields
                                                        .iter()
                                                        .find(|(field_key, _)| *field_key == k)
                                                        .map(|(_, field_def)| field_def.name.clone())
                                                        .unwrap_or_else(|| k.clone());
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
                                    when=move || !st.meta.get().unwrap().operations.is_empty()
                                    fallback=|| {
                                        view! {
                                            <div class="gr">
                                                <h5>"No saved operations"</h5>
                                            </div>
                                        }
                                    }
                                >
                                    {move || {
                                        match transform_fields(&st.meta.get().unwrap().fields, &data.get().unwrap().fields) {
                                            Err(e) => view! { <p>{e}</p> }.into_any(),
                                            Ok(vars) => {
                                                st.meta
                                                    .get()
                                                    .unwrap()
                                                    .operations
                                                    .iter()
                                                    .map(|op| { view! { <RunOper oper=op.clone() vars=vars.clone() /> }.into_any() })
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
                    _ => view! { <span class="gr">{t!(i18n, all.err_unknown)}</span> }.into_any(),
                }
            }}
        </Show>
    }
}

#[component]
pub fn RunOper(oper: Operation, vars: HashMap<String, f64>) -> impl IntoView {
    let i18n = use_i18n();
    use exmex::Express;
    let st = use_context::<State>().expect("State missing");
    let orig = RwSignal::new(vars.clone());
    let (inner, set_inner) = signal(vars.clone());
    let local_prec = RwSignal::new(oper.prec);

    // Парсинг формулы
    let formula = oper.expr.clone();
    let expr_result = Memo::new(move |_| exmex::parse::<f64>(&formula));

    let var_names = Memo::new(move |_| match expr_result.get() {
        Ok(e) => e.var_names().iter().map(|n| n.to_string()).collect::<Vec<_>>(),
        Err(_) => vec![],
    });

    let calculation = Memo::new(move |_| match expr_result.get() {
        Ok(e) => {
            let current_inputs = inner.get();
            let mut vals = Vec::new();

            for raw_name in e.var_names() {
                if let Some(val) = current_inputs.get(&raw_name.to_string()) {
                    vals.push(*val);
                } else {
                    return Err(raw_name.to_string());
                }
            }

            match e.eval(&vals) {
                Ok(result) => {
                    if result.is_infinite() {
                        Err("∞".to_string())
                    } else if result.is_nan() {
                        Err("?".to_string())
                    } else {
                        Ok(result)
                    }
                }
                Err(e) => Err(format!("{:?}", e)),
            }
        }
        Err(e) => Err(format!("{}", e)),
    });

    let filtered = move || {
        let names = var_names.get();
        let inner = orig.get();
        names.into_iter().filter(|n| !inner.contains_key(n)).collect::<Vec<String>>()
    };

    let on_input = move |ev: web_sys::Event| {
        let new_val = event_target_value(&ev).parse().unwrap_or(2);
        local_prec.set(new_val);
        let pb = st.get_full_pb(st.selected.get_untracked().refer.unwrap());

        let mut for_send = st.meta.get_untracked().unwrap().operations;
        let operation = for_send.iter_mut().find(|o| o.id == oper.id).unwrap();
        operation.prec = new_val;

        spawn_local(async move {
            let _ = invoke(
                "update_meta_entity",
                &tauri_args! {
                    "pb": pb,
                    "key": "operations",
                    "value": for_send
                },
            )
            .await;
        });
    };

    Effect::new(move |_| {
        log::info!("filtered: {:#?}", filtered());
        log::info!("inner {:?}", inner.get());
    });

    view! {
        <div class="oper gr grid center gap04">
            <strong class="center">
                {oper.name}" : "
                <span>
                    {move || match calculation.get() {
                        Ok(val) => view! { <span class="success">{format!("{:.*}", local_prec.get() as usize, val)}</span> }.into_any(),
                        Err(e) => view! { <span class="warn">{e}</span> }.into_any(),
                    }}
                </span>
                <span class="ml1">{t!(i18n,ref_main.prec)}</span>
                <select class="precision" on:change=on_input prop:value=move || local_prec.get()>
                    {(0..18).map(|n| view! { <option value=n>{n}</option> }).collect_view()}
                </select>

            </strong>

            // Динамические инпуты для неопределенных переменных
            <div class="gline">
                <For
                    each=move || filtered()
                    key=|name| name.clone()
                    children=move |name| {
                        let n_input = name.clone();
                        let (display_val, set_display_val) = signal(inner.get_untracked().get(&name).cloned().unwrap_or(0.0).to_string());
                        view! {
                            <label class="m0">{name.clone()}</label>
                            <input
                                type="text"
                                // Теперь это сработает, так как display_val будет меняться всегда
                                class:error=move || {
                                    let val = display_val.get();
                                    if val.is_empty() || val == "." || val == "-" {
                                        return false;
                                    }
                                    val.parse::<f64>().is_err()
                                }
                                prop:value=display_val
                                on:input={
                                    let value_name = n_input.clone();
                                    move |ev| {
                                        let val_str = event_target_value(&ev);
                                        set_display_val.set(val_str.clone());
                                        if let Ok(val) = val_str.parse::<f64>() {
                                            set_inner
                                                .update(|map| {
                                                    map.insert(value_name.clone(), val);
                                                });
                                        }
                                    }
                                }
                            />
                            <button
                                class="bf m0"
                                type="button"
                                on:click=move |_| {
                                    set_display_val.set("0".to_string());
                                    set_inner
                                        .update(|map| {
                                            map.insert(n_input.clone(), 0.0);
                                        });
                                }
                            >
                                "🧹"
                            </button>
                        }
                    }
                />
            </div>

            <small>{oper.desc}</small>
            <small>{oper.expr}</small>
        </div>
    }
}

// Получить заголовок элемента для списка/заголовка
pub fn get_item_title(record: &DataRecord, meta: &TableMeta) -> String {
    let fields_text = if !meta.search_config.is_empty() {
        meta.search_config
            .iter()
            .filter_map(|field| record.fields.get(field))
            .filter(|v| !v.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" • ")
    } else {
        String::new()
    };

    if fields_text.is_empty() {
        format!("#{}", record.id)
    } else {
        format!("{} • #{}", fields_text, record.id)
    }
}

pub fn transform_fields(
    fields_def: &HashMap<String, FieldDef>, data_fields: &HashMap<String, String>,
) -> Result<HashMap<String, f64>, String> {
    // Создаём HashMap для быстрого поиска по ключу
    let field_types_map: HashMap<_, _> = fields_def.iter().map(|(k, def)| (k.clone(), def.ftype.as_str())).collect();

    let field_names_map: HashMap<_, _> = fields_def.iter().map(|(k, def)| (k.clone(), def.name.as_str())).collect();

    let (successes, errors): (Vec<_>, Vec<_>) = data_fields
        .iter()
        // только числовые поля
        .filter(|(k, _)| field_types_map.get(*k).copied() == Some("number"))
        .map(|(k, v)| {
            let name = field_names_map.get(k).cloned().unwrap_or(k.as_str()).to_string();
            v.parse::<f64>().map(|val| (name.clone(), val)).map_err(|_| name)
        })
        .partition(Result::is_ok);

    if !errors.is_empty() {
        let err_names: Vec<String> = errors.into_iter().map(Result::unwrap_err).collect();
        return Err(format!("Invalid number format in fields: {}", err_names.join(", ")));
    }

    Ok(successes.into_iter().map(Result::unwrap).collect())
}

/*
Было:
```rust
let names = table_meta.clone().field_names;
```

Стало (если `fields: HashMap<String, FieldDef>`):

```rust
// Получить все имена полей:
let names: Vec<String> = table_meta.fields.iter()
    .map(|(_, field_def)| field_def.name.clone())
    .collect();

// Или если нужно с ключами (f_0, f_1...):
let fields_with_keys = table_meta.fields.clone();

// Для конкретного поля по ключу:
if let Some((_, field_def)) = table_meta.fields.iter().find(|(k, _)| k == &field_key) {
    let display_name = &field_def.name;
}
```

**Для быстрого доступа по ключу** (как раньше с HashMap), создайте локальный хешмап:

```rust
let field_names_map: HashMap<String, String> = table_meta.fields.iter()
    .map(|(k, v)| (k.clone(), v.name.clone()))
    .collect();

let name = field_names_map.get("f_0");
```

*/
