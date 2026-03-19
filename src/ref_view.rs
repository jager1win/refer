use crate::{app::*, functions::*, i18n::*, tauri_args};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::path::PathBuf;

#[component]
pub fn Ref_main() -> impl IntoView {
    let i18n = use_i18n();
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("reference not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
    let query_string = RwSignal::new("".to_string());
    let meta = RwSignal::new(MetaState::Pending);
    let data = RwSignal::new(None::<Vec<DataRecord>>);
    let selected_el = RwSignal::new(None::<u32>);

    // get meta
    spawn_local(async move {
        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
        match invoke("get_meta", &tauri_args!("pb": pb)).await {
            Ok(js) => {
                let s = from_value::<TableMeta>(js).unwrap();
                meta.set(MetaState::Loaded(s))
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("{} {}", "", &error_msg));
                meta.set(MetaState::Invalid(error_msg));
            }
        };
    });

    // get data. init - first 10 el. then - by search
    let search_items = move |pb: PathBuf, query: String| {
        spawn_local(async move {
            match invoke("search_items", &tauri_args!("pb": pb, "query": query)).await {
                Ok(js) => {
                    let s = from_value::<Vec<DataRecord>>(js).unwrap();
                    data.set(Some(s));
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!("{} {}", "", error_msg));
                }
            };
        });
    };

    let save_search_config = move |field: String| {
        let pb = full_pb(stat.get_untracked().db_path, selected_ref.get_untracked().unwrap());
        meta.update(|state| {
            if let MetaState::Loaded(meta) = state {
                if let Some(pos) = meta.search_config.iter().position(|f| f == &field) {
                    meta.search_config.remove(pos);
                } else {
                    meta.search_config.push(field);
                }
            }
        });

        spawn_local({
            let meta = meta.get_untracked();
            let MetaState::Loaded(meta) = meta else { return };
            async move {
                let _ = invoke(
                    "save_search_config",
                    &tauri_args! {
                        "pb": pb,
                        "vec": meta.search_config
                    },
                )
                .await;
            }
        });
    };

    Effect::new(move |_| {
        log::info!("meta: {:#?}", meta.get());
    });

    // debounce for search. run if upd query_string || meta
    Effect::new(move |_| {
        let _ = query_string.get();
        let _ = meta.get();

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
    view! {
        <Show when=move || { selected_el.get().is_none() } fallback=|| view! { <Ref_el /> }>
            <div class="ref">
                {move || {
                    match meta.get() {
                        MetaState::Invalid(msg) => view! { <h5 class="error gr">"🚫 "{msg}</h5> }.into_any(),
                        MetaState::Pending => view! { <span class="gr" aria-busy="true"></span> }.into_any(),
                        MetaState::Loaded(table_meta) => {
                            let meta_for = sort_f_keys_v(table_meta.clone().search_config);
                            view! {
                                <div class="gr">
                                    <div class="header-row">
                                        <button on:click=move |_| selected_ref.set(None)>"←"</button>

                                        <div class="title-group">
                                            <span>
                                                {move || remove_refer_ext(&selected_ref.get().unwrap_or_default())}
                                            </span>
                                            <small class="meta grid">
                                                <span>{table_meta.name.clone()}</span>
                                                <span>{table_meta.desc.clone()}</span>
                                            </small>
                                        </div>

                                        <button on:click=move |_| edit_ref.set(true)>"✎"</button>
                                    </div>

                                    // input. remove if empty search_config or count = 0
                                    {
                                        let metaclon = meta_for.clone();
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
                                    {match meta_for.clone().is_empty() {
                                        true => view! { <div class="search_results">""</div> }.into_any(),
                                        false => {
                                            let col_count = meta_for.clone().len();
                                            let names = table_meta.clone().field_names;
                                            view! {
                                                <div class="search_results" style=format!("--cols: {}", col_count)>
                                                    <div class="row">
                                                        {{
                                                            f2name_v(meta_for.clone(), names)
                                                                .into_iter()
                                                                .map(|n| view! { <small>{n}</small> })
                                                                .collect_view()
                                                        }}
                                                    </div>
                                                    <For
                                                        each=move || data.get().unwrap_or_default()
                                                        key=|rec: &DataRecord| rec.id
                                                        children=move |rec: DataRecord| {
                                                            let search_fields = meta_for.clone();

                                                            view! {
                                                                <button class="row" on:click=move |_| selected_el.set(Some(rec.id))>
                                                                    {search_fields
                                                                        .into_iter()
                                                                        .filter_map(|k| { rec.fields.get(&k).map(|v| (k, v.clone())) })
                                                                        .map(|(_k, v)| {
                                                                            let val_str = match v {
                                                                                FieldValue::Text(s) => s,
                                                                                FieldValue::Number(n) => n.to_string(),
                                                                                FieldValue::Null => String::new(),
                                                                            };
                                                                            view! { <div>{val_str}</div> }
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
                                                                        <span>{format!("{:?}", ft)}</span>
                                                                        <label>
                                                                            <input
                                                                                type="checkbox"
                                                                                prop:checked={
                                                                                    let k = k.clone();
                                                                                    move || {
                                                                                        meta.with(|state| {
                                                                                            match state {
                                                                                                MetaState::Loaded(inner_meta) => {
                                                                                                    inner_meta.search_config.contains(&k)
                                                                                                }
                                                                                                _ => false,
                                                                                            }
                                                                                        })
                                                                                    }
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
                                                    oper.into_iter()
                                                        .map(|k| {
                                                            view! { <div>{k.name}" : "{k.expression}</div> }
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
                    }
                }}
            </div>
        </Show>
    }
}

#[component]
pub fn Ref_el() -> impl IntoView {
    let selected_el = use_context::<RwSignal<Option<u32>>>().expect("element not found");
    let edit_el = RwSignal::new(false);
    view! {
        <div class="ref_el">
            <div class="gr">
                <h3 class="ffull ">
                    <button on:click=move |_| selected_el.set(None)>"←"</button>
                    Ref element
                    {move || selected_el.get()}
                    <button on:click=move |_| edit_el.set(true)>"✎"</button>
                </h3>
            </div>
        </div>
    }
}
