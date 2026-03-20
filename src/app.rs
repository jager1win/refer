use crate::{functions::*, i18n::*, ref_view::*};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::PathBuf;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: &JsValue) -> Result<JsValue, JsValue>;
}

#[macro_export]
macro_rules! tauri_args {
    ($($key:literal : $val:expr),* $(,)?) => {{
        ::serde_wasm_bindgen::to_value(&::leptos::serde_json::json!({ $($key: $val),* })).unwrap()
    }};
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    theme: String,
    language: String,
    color: String,
    log: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsState {
    pub db_path: PathBuf,
    pub db_path_size: u64,
    pub db_list: Vec<PathBuf>,
    pub log_path: String,
    pub db_path_ok: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
struct CreateForm {
    mode: String,                   // "empty", "sheet", "sqlite"
    db_name: PathBuf,               // имя БД
    has_header: bool,               // есть заголовок
    file_extension: Option<String>, // расширение файла
    file_path: Option<PathBuf>,     // содержимое файла
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    pub id: u32,
    pub fields: HashMap<String, FieldValue>, // f_0, f_1 и т.д.
}

// Разные типы полей для гибкой обработки
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Null,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TableMeta {
    pub name: String,
    pub desc: String,
    pub field_names: HashMap<String, String>,
    pub field_types: HashMap<String, FieldType>,
    pub operations: Vec<Operation>,
    pub search_config: Vec<String>,
    pub count_data: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Operation {
    pub name: String, // "Total Price"
    pub description: String,
    pub expression: String, // "f_6 * 17 / f_20"
}

#[derive(Clone, Debug)]
pub enum MetaState {
    Pending,
    Loaded(TableMeta),
    Invalid(String), // причина невалидности
}

#[component]
pub fn App() -> impl IntoView {
    let settings: RwSignal<AppSettings> = RwSignal::new(AppSettings {
        theme: "light".into(),
        language: "en".into(),
        color: "blue".into(),
        log: "false".into(),
    });
    let stat: RwSignal<StatisticsState> = RwSignal::new(StatisticsState::default());
    let selected_ref: RwSignal<Option<PathBuf>> = RwSignal::new(None::<PathBuf>);
    let edit_ref: RwSignal<bool> = RwSignal::new(false);
    let active_tab: RwSignal<i32> = RwSignal::new(1);
    let now: RwSignal<String> = RwSignal::new(String::from(""));
    let er_pat = ["fail", "error", "warning", "invalid", "unknown"];

    provide_context(settings);
    provide_context(stat);
    provide_context(selected_ref);
    provide_context(edit_ref);
    provide_context(now);
    provide_context(active_tab);
    leptos_meta::provide_meta_context();

    // set settings from file
    spawn_local(async move {
        match invoke("get_settings", &JsValue::NULL).await {
            Ok(js) => {
                let s = from_value::<AppSettings>(js).unwrap_or_default();
                settings.set(s);
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("{} {}", "Failed init settings:", error_msg));
            }
        };
    });

    upd_stat(stat, now);

    let clean = move || {
        selected_ref.set(None::<PathBuf>);
        edit_ref.set(false);
        now.set(String::from(""))
    };

    // now clean if not error+
    Effect::new(move |_| {
        let cur = now.get();
        let contains_bad = er_pat.iter().any(|p| cur.to_lowercase().contains(p));
        if !contains_bad {
            let cur_clone = cur.clone();
            set_timeout(
                move || {
                    if now.get() == cur_clone {
                        now.set("".to_string());
                    }
                },
                std::time::Duration::from_millis(6000),
            );
        }
    });

    view! {
        <header>
            <nav class="top-nav">
                <button
                    class="navb"
                    class:active=move || active_tab.get() == 0
                    on:click=move |_| {
                        if active_tab.get() == 0 {
                            active_tab.set(1);
                        } else {
                            active_tab.set(0);
                        }
                    }
                >
                    "⚙"
                </button>
                <button
                    class="navb"
                    class:active=move || active_tab.get() == 1
                    on:click=move |_| {
                        upd_stat(stat, now);
                        clean();
                        active_tab.set(1);
                    }
                >
                    "📚 "
                    <span>
                        {
                            let i18n = use_i18n();
                            t!(i18n, all.references)
                        }
                    </span>
                </button>
                <button
                    class="navb"
                    class:active=move || active_tab.get() == 2
                    on:click=move |_| {
                        upd_stat(stat, now);
                        clean();
                        if active_tab.get() == 2 {
                            active_tab.set(1);
                        } else {
                            active_tab.set(2);
                        }
                    }
                >
                    "✚"
                </button>
            </nav>
            <div class="now" class:error=move || er_pat.iter().any(|p| now.get().to_lowercase().contains(p))>
                <p>{move || now.get()}</p>
                <span class="sp_close" class:hidden=move || now.get().is_empty() on:click=move |_| now.set("".to_string())>
                    "x"
                </span>
            </div>
        </header>
        <main class="main">
            // Основной слой — всегда активен
            <div class="main-content">
                <ReferencesContainer />
            </div>

            // Слой Настроек
            <div class="tab-content" class:active=move || active_tab.get() == 0>
                <Settings />
            </div>

            // Слой Создания
            <div class="tab-content" class:active=move || active_tab.get() == 2>
                <Create />
            </div>
        </main>
        {move || match settings.get().log.as_str() {
            "false" => view! { "" }.into_any(),
            "true" => view! { <LogViewer /> }.into_any(),
            _ => view! { "" }.into_any(),
        }}
    }
}

#[component]
fn ReferencesContainer() -> impl IntoView {
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("edit not found");

    view! {
        <div class="references-container">
            <Show when=move || selected_ref.get().is_some() fallback=|| view! { <Refs /> }>
                <Show when=move || !edit_ref.get() fallback=|| view! { <Edit /> }>
                    <Ref_main />
                </Show>
            </Show>
        </div>
    }
}

#[component]
fn Settings() -> impl IntoView {
    let i18n = use_i18n();
    let all: &[Locale] = Locale::get_all();
    let settings = use_context::<RwSignal<AppSettings>>().expect("settings not found");
    let colors = [
        "orange", "lime", "green", "cyan", "blue", "indigo", "purple", "fuchsia", "pink", "rose", "slate", "zinc",
        "taupe", "mauve", "mist", "olive",
    ];
    /* Possible color choices: orange, lime, green, cyan, blue, indigo, purple, fuchsia, pink, rose, slate, zinc, taupe, mauve, mist, olive*/

    let toggle_theme = move |_| {
        settings.update(|current| {
            if current.theme == "light" {
                current.theme = "dark".to_string();
            } else {
                current.theme = "light".to_string();
            }
            spawn_local(async move {
                let _ = invoke("set_settings", &tauri_args!("new": settings.get_untracked())).await;
            });
        });
    };

    let toggle_log = move |_| {
        settings.update(|current| {
            if current.log == "false" {
                current.log = "true".to_string();
            } else {
                current.log = "false".to_string();
            }
            spawn_local(async move {
                let _ = invoke("set_settings", &tauri_args!("new": settings.get_untracked())).await;
            });
        });
    };

    let set_color = move |color: &str| {
        settings.update(|current| {
            current.color = color.to_string();
            spawn_local(async move {
                let _ = invoke("set_settings", &tauri_args!("new": settings.get_untracked())).await;
            });
        });
    };

    Effect::new(move |_| {
        let lang_code = settings.get().language;
        if let Some(&loc) = all
            .iter()
            .find(|l| l.to_string() == lang_code)
            .or_else(|| all.iter().find(|l| l.to_string() == "en"))
            .or(all.first())
        {
            i18n.set_locale(loc);
        }
    });

    Effect::new(move |_| {
        let theme_value = settings.get().theme;
        let color_value = settings.get().color;
        let document = window().document().unwrap();
        let html_element = document.document_element().unwrap();
        html_element.set_attribute("data-theme", &theme_value).unwrap();
        html_element.set_attribute("data-color", &color_value).unwrap();
    });

    //log::debug!("lang: {:?}", &all);
    //log::debug!("lang: {:?}", &current);
    view! {
        <div class="settings_block gr">
            <h5>{t!(i18n, settings.title)}</h5>

            <div class="locale-switcher gridl">
                <span>{t!(i18n, settings.language)}</span>
                <span class="ar_lang">
                    {all
                        .iter()
                        .map(move |&loc| {
                            let code = loc.as_str();
                            let is_active = move || settings.get().language == loc.to_string();
                            view! {
                                <button
                                    class=move || { if is_active() { "locale-btn active".to_string() } else { "locale-btn".to_string() } }
                                    on:click=move |_| {
                                        if !is_active() {
                                            spawn_local(async move {
                                                i18n.set_locale(loc);
                                                settings
                                                    .update(|current| {
                                                        current.language = loc.as_str().to_string();
                                                    });
                                                let _ = invoke("set_settings", &tauri_args!("new": settings.get_untracked())).await;
                                            });
                                        }
                                    }
                                >
                                    {code}
                                </button>
                            }
                        })
                        .collect_view()}
                </span>
            </div>

            <div class="color-switcher gridl">
                <span>{t!(i18n, settings.color)}</span>
                <span class="ar_color">
                    {colors
                        .iter()
                        .map(move |&color| {
                            view! {
                                <button
                                    class=move || {
                                        if settings.get().color == color { format!("{} active", color) } else { color.to_string() }
                                    }
                                    on:click=move |_| set_color(color)
                                />
                            }
                        })
                        .collect_view()}
                </span>
            </div>

            <div class="gridl">
                <div>
                    <span>{t!(i18n, settings.theme)}</span>
                </div>
                <div>
                    <button on:click=toggle_theme class="theme-switcher">
                        {move || match settings.get().theme.as_str() {
                            "light" => "🌙",
                            "dark" => "🌞",
                            _ => "🌞",
                        }}
                    </button>
                </div>
            </div>

            <div class="gridl">
                <div>
                    <span>{t!(i18n, settings.logs)}</span>
                </div>
                <div>
                    <button on:click=toggle_log class="theme-switcher">
                        {move || match settings.get().log.as_str() {
                            "false" => t!(i18n, settings.show).into_any(),
                            "true" => t!(i18n, settings.hide).into_any(),
                            _ => "".into_any(),
                        }}
                    </button>
                </div>
            </div>
        </div>
        <div class="settings_block gr">
            <p class="gridl"><span>"Github"</span><a href="https://github.com/jager1win/refer" target="_blank">"https://github.com/jager1win/refer"</a></p>
        </div>
    }
}

#[component]
fn Refs() -> impl IntoView {
    let i18n = use_i18n();
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let patterns = ["fail", "error"];

    view! {
        <div class="refs-list grid2 gr">
            <For
                each=move || stat.get().db_list.clone()
                key=|item| item.clone()
                children=move |item: PathBuf| {
                    view! {
                        <button class="rlist" on:click=move |_| { selected_ref.set(Some(item.clone())) }>
                            {remove_refer_ext(&item)}
                        </button>
                    }
                }
            />
        </div>

        <div class="grid1a stat_table gr info">
            <b>{t!(i18n, refs.st_path)}": "</b>
            <span>{move || stat.get().db_path.display().to_string()}</span>
            <b>{t!(i18n, refs.st_access)}": "</b>
            <span class:error=move || {
                patterns.iter().any(|p| stat.get().db_path_ok.to_lowercase().contains(p))
            }>{move || stat.get().db_path_ok}</span>
            <b>{t!(i18n, refs.st_count)}": "</b>
            <span>{move || stat.get().db_list.len()}</span>
            <b>{t!(i18n, refs.st_size)}": "</b>
            <span>{move || read_size(stat.get().db_path_size)}</span>
            <b>{t!(i18n, refs.st_log)}": "</b>
            <span>{move || stat.get().log_path}</span>
        </div>
    }
}

#[component]
fn Edit() -> impl IntoView {
    let i18n = use_i18n();
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let edit_ref = use_context::<RwSignal<bool>>().expect("edit not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");

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
    view! {
        <h3>{t!(i18n, edit.test)}</h3>

        <button on:click=move |_| { edit_ref.set(false) }>"✎ Save "</button>
        <button on:click=move |_| del_ref(selected_ref.get().unwrap())>{t!(i18n, all.del)}</button>
    }
}

#[component]
fn Create() -> impl IntoView {
    use leptos::ev::SubmitEvent;
    let i18n = use_i18n();
    let stat = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now = use_context::<RwSignal<String>>().expect("now not found");
    let active_tab = use_context::<RwSignal<i32>>().expect("active_tab not found");
    let selected_ref = use_context::<RwSignal<Option<PathBuf>>>().expect("selected not found");
    let err_form: RwSignal<String> = RwSignal::new("".to_string());
    let is_loading = RwSignal::new(false);
    // mode: "empty" | "sheet" | "sqlite"
    let mode = RwSignal::new("sheet".to_string());
    let form_ref = NodeRef::<leptos::html::Form>::new();
    // send create
    let submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        if is_loading.get() {
            return;
        }
        let Some(form) = form_ref.get() else { return };

        // 1. СОБИРАЕМ ДАННЫЕ
        let mut form_data = CreateForm {
            mode: mode.get(),
            ..Default::default()
        };

        let elements = form.elements();
        for i in 0..elements.length() {
            if let Some(element) = elements.item(i)
                && let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>()
            {
                match input.name().as_str() {
                    "db_name" => form_data.db_name = input.value().into(),
                    "has_header" => form_data.has_header = input.checked(),
                    _ => {}
                }
            }
        }

        // 2. проверка - поле имя файла
        // is empty
        if form_data.db_name.as_os_str().is_empty() {
            err_form.set(format!("🖉 {}", t_string!(i18n, create.fname)));
            return;
        }

        // check simbols
        match validate_relative_refer_path(&form_data.db_name) {
            Ok(()) => log::debug!("simbols: ok"),
            Err(()) => {
                err_form.set(format!("🖉 {}", t_string!(i18n, create.fname)));
                return;
            }
        };

        // if exist
        if !form_data.db_name.to_string_lossy().ends_with(".refer") {
            let _ = form_data.db_name.set_extension("refer");
        }

        if stat.get().db_list.contains(&form_data.db_name) {
            err_form.set(format!("🖉 !exist {}", t_string!(i18n, create.fname)));
            return;
        }

        let command_name = if form_data.mode == "empty" {
            "create_empty"
        } else {
            "create_from_file"
        };
        now.set("⏳".to_string());
        is_loading.set(true);
        spawn_local(async move {
            match invoke(command_name, &tauri_args!("val": form_data)).await {
                Ok(_s) => {
                    log::info!("ok return");
                    now.set(format!(
                        "{}: {}",
                        tu_string!(i18n, create.ok_create),
                        form_data.db_name.to_string_lossy()
                    ));
                    if let Some(f) = form_ref.get_untracked() {
                        f.reset();
                    }
                    is_loading.set(false);
                    selected_ref.set(Some(form_data.db_name));
                    active_tab.set(1);
                }
                Err(js) => {
                    is_loading.set(false);
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!(
                        "{}: {} - {}",
                        tu_string!(i18n, create.er_create),
                        form_data.db_name.to_string_lossy(),
                        error_msg
                    ))
                }
            }
        });
    };
    // send create examle
    let create_example = move |name: PathBuf| {
        let form_data = CreateForm {
            mode: "example".to_string(),
            db_name: name.to_path_buf(),
            ..Default::default()
        };
        spawn_local(async move {
            match invoke("create_example", &tauri_args!("val": form_data)).await {
                Ok(_js) => {
                    now.set(format!("{}: {:?}", tu_string!(i18n, create.ok_create), &name));
                    upd_stat(stat, now);
                    selected_ref.set(Some(name));
                    active_tab.set(1);
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    now.set(format!(
                        "{}: {:?} - {}",
                        tu_string!(i18n, create.er_create),
                        &name,
                        error_msg
                    ));
                }
            }
        });
    };

    Effect::new(move |_| {
        err_form.track();
        set_timeout(
            move || err_form.set("".to_string()),
            std::time::Duration::from_millis(6000),
        );
    });
    view! {
        <Show
            when=move || stat.get().db_path_ok == "Ok"
            fallback=move || {
                view! {
                    <p>{tu!(i18n, create.main_error)}":"</p>
                    <span>{move || stat.get().db_path.display().to_string()}</span>
                    <span class="error">{move || stat.get().db_path_ok}</span>
                }
            }
        >
            <div class="gr">
                <span class="err_send">{move || err_form.get()}</span>
                <form class="form_new" on:submit=submit node_ref=form_ref novalidate>
                    <fieldset class="grida m0">
                        <label>
                            <input
                                type="radio"
                                name="mode"
                                value="sheet"
                                checked=move || mode.get() == "sheet"
                                on:change=move |_| mode.set("sheet".to_string())
                            />
                            {tu!(i18n, create.ref_from_table)}
                        </label>
                        <label>
                            <input
                                type="radio"
                                name="mode"
                                value="sqlite"
                                checked=move || mode.get() == "sqlite"
                                on:change=move |_| mode.set("sqlite".to_string())
                            />
                            {tu!(i18n, create.ref_from_db)}
                        </label>
                        <label>
                            <input
                                type="radio"
                                name="mode"
                                value="empty"
                                checked=move || mode.get() == "empty"
                                on:change=move |_| mode.set("empty".to_string())
                            />
                            {tu!(i18n, create.ref_from_empty)}
                        </label>
                    </fieldset>

                    {move || {
                        match mode.get().as_str() {
                            "empty" => view! { <h6>{tu!(i18n, create.ttp_empty)}</h6> }.into_any(),
                            "sheet" => {
                                view! {
                                    <h6>{tu!(i18n, create.ftable)}</h6>
                                    <h6>{tu!(i18n, create.ttp_table)}</h6>
                                    <div class="gridl">
                                        <label>
                                            {t!(i18n, create.fheader)}
                                            <span data-placement="right" data-tooltip=t_string!(i18n, create.ttp_header)>
                                                "?"
                                            </span>
                                        </label>
                                        <input type="checkbox" class="" name="has_header" checked />
                                    </div>
                                }
                                    .into_any()
                            }
                            "sqlite" => {
                                view! {
                                    <h6>{tu!(i18n, create.fsqlite)}</h6>
                                    <h6>{tu!(i18n, create.ttp_sqlite)}</h6>
                                }
                                    .into_any()
                            }
                            _ => view! { <div></div> }.into_any(),
                        }
                    }}

                    <div class="block gridl">
                        <label>
                            {t!(i18n, create.fname)} <span data-placement="right" data-tooltip=t_string!(i18n, create.ttp_path)>
                                "?"
                            </span>
                        </label>

                        <input type="text" name="db_name" placeholder="my_refer" required />
                    </div>

                    <div class="actions">
                        <button type="submit" disabled=move || is_loading.get()>
                            {move || {
                                if mode.get() == "empty" {
                                    { t!(i18n, create.button) }.into_any()
                                } else {
                                    { t!(i18n, create.button_file) }.into_any()
                                }
                            }}
                        </button>
                    </div>
                </form>
            </div>

            <div class="gr">
                <h5>{t!(i18n, create.example)}</h5>
                <p class="center">{t!(i18n, create.example_replace)}</p>
                <div class="test_create gridl">
                    <button on:click=move |_| create_example(PathBuf::from("example/ballistics.refer"))>"Ballistics Data"</button>
                    <span>{t!(i18n, create.example_desc_1)}</span>
                    <button on:click=move |_| create_example(PathBuf::from("example/222.refer"))>"222"</button>
                    <span>{t!(i18n, create.example_desc_2)}</span>
                    <button on:click=move |_| create_example(PathBuf::from("example/333.refer"))>"333"</button>
                    <span>{t!(i18n, create.example_desc_3)}</span>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn LogViewer() -> impl IntoView {
    let logs = RwSignal::new(None::<String>);
    let show_logs = RwSignal::new(false);

    let load_logs = move || {
        spawn_local(async move {
            match invoke("get_log", &JsValue::NULL).await {
                Ok(js) => {
                    let res = from_value::<String>(js).unwrap_or_default();
                    logs.set(Some(res));
                }
                Err(js) => {
                    let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                    logs.set(Some(format!("Failed to load logs: {}", error_msg)));
                }
            }
        });
    };

    Effect::new(move |_| {
        if show_logs.get() {
            load_logs();
            #[allow(clippy::redundant_closure)]
            let _ = set_interval_with_handle(move || load_logs(), std::time::Duration::from_secs(2)).ok();
        }
    });

    view! {
        <div class="log-viewer-wrapper" class:active=move || show_logs.get()>
            <button class="log-trigger-btn" on:click=move |_| show_logs.update(|v| *v = !*v)>
                "📋"
            </button>

            <div class="log-panel">
                <code id="logs">{move || logs.get()}</code>
            </div>
        </div>
    }
}
