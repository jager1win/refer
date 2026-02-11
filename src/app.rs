use leptos::{prelude::*};
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use crate::i18n::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    theme: String,
    language: String,
}

#[derive(Serialize)]
struct SettingsBack {
    new: AppSettings,
}


#[derive(Serialize)]
struct ToBack { val: String }

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct StatisticsState {
    pub db_path: String,
    pub db_path_size: u64,
    pub db_list: Vec<String>,
    pub log_path: String,
    pub db_path_ok: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
struct CreateForm {
    mode: String,                    // "empty", "sheet", "sqlite"
    db_name: String,                 // имя БД
    csv_delim: String,               // разделитель CSV
    has_header: bool,                // есть заголовок
    file_extension: Option<String>,  // расширение файла
    file_data: Option<Vec<u8>>,      // содержимое файла
}

#[derive(Serialize)]
struct CreateFormBack { val: CreateForm }

#[component]
pub fn App() -> impl IntoView {
    let settings: RwSignal<AppSettings> = RwSignal::new(AppSettings {theme: "light".into(),language: "en".into()});
    let stat: RwSignal<StatisticsState> = RwSignal::new(StatisticsState::default());
    let selected_ref:RwSignal<Option<String>>  = RwSignal::new(None::<String>);
    let edit_ref: RwSignal<bool> = RwSignal::new(false);
    let active_tab: RwSignal<i32> = RwSignal::new(2);
    let now: RwSignal<String> = RwSignal::new(String::from(""));
    let er_pat = ["fail", "error"];

    provide_context(settings);
    provide_context(stat);
    provide_context(selected_ref);
    provide_context(edit_ref);
    provide_context(now);
    provide_context(active_tab);
    leptos_meta::provide_meta_context();

    // set settings from file
    spawn_local(async move {
        let js = invoke("get_settings", JsValue::NULL).await;
        match from_value::<AppSettings>(js) {
            Ok(s) => settings.set(s),
            Err(e) => {
                set_now(now, format!("{} {}", "Failed init settings:", e));
            }
        };
    });

    upd_stat(stat,now);

    let clean = move || {
        selected_ref.set(None::<String>);
        edit_ref.set(false);
    };
    
    view! {
        <nav class="top-nav">
            <button
                class="navb"
                class:active=move || active_tab.get() == 0
                on:click=move |_| {
                    clean();
                    active_tab.set(0)
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
                {
                    let i18n = use_i18n();
                    t!(i18n, all.references)
                }
                {move || selected_ref.get()}
            </button>
            <button
                class="navb"
                class:active=move || active_tab.get() == 2
                on:click=move |_| {
                    upd_stat(stat, now);
                    clean();
                    active_tab.set(2)
                }
            >
                "✚"
            </button>
        </nav>
        <div
            class="grid now"
            class:error=move || er_pat.iter().any(|p| now.get().to_lowercase().contains(p))
        >
            <p class="">{move || now.get()}</p>
        </div>
        <main class="main">
            <div class="tab-content" class:active=move || active_tab.get() == 0>
                <Settings />
            </div>
            <div class="tab-content" class:active=move || active_tab.get() == 1>
                <ReferencesContainer />
            </div>
            <div class="tab-content" class:active=move || active_tab.get() == 2>
                <Create />
            </div>

        </main>
    }
}

#[component]
fn ReferencesContainer() -> impl IntoView {
    let selected_ref: RwSignal<Option<String>> = use_context::<RwSignal<Option<String>>>().expect("selected not found");
    let edit_ref: RwSignal<bool> = use_context::<RwSignal<bool>>().expect("edit not found");

    view! {
        <div class="references-container">
            <Show when=move || selected_ref.get().is_some() fallback=|| view! { <Refs /> }>
                <Show when=move || !edit_ref.get() fallback=|| view! { <Edit /> }>
                    <Ref />
                </Show>
            </Show>
        </div>
    }
}

#[component]
fn Settings() -> impl IntoView {
    let i18n = use_i18n();
    let all: &[Locale] = Locale::get_all();
    let settings: RwSignal<AppSettings> = use_context::<RwSignal<AppSettings>>().expect("settings not found");

    //let current = move || i18n.get_locale();
    let toggle_theme = move |_| {
        settings.update(|current| {
            if current.theme == "light" {
                current.theme = "dark".to_string();
            } else {
                current.theme = "light".to_string();
            }
            spawn_local(async move {
                let args = to_value(&SettingsBack {new: settings.get_untracked()}).unwrap();
                let _ = invoke("set_settings", args).await;
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
        let document = window().document().unwrap();
        let html_element = document.document_element().unwrap();
        html_element
            .set_attribute("data-theme", &theme_value)
            .unwrap();
    });

    //log::debug!("lang: {:?}", &all);
    //log::debug!("lang: {:?}", &current);
    view! {
        <div class="settings_block">
            <h3>{t!(i18n, settings.title)}</h3>

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
                                    class=move || {
                                        if is_active() {
                                            "locale-btn active".to_string()
                                        } else {
                                            "locale-btn".to_string()
                                        }
                                    }
                                    on:click=move |_| {
                                        if !is_active() {
                                            spawn_local(async move {
                                                i18n.set_locale(loc);
                                                settings
                                                    .update(|current| {
                                                        current.language = loc.as_str().to_string();
                                                    });
                                                let args = to_value(
                                                        &SettingsBack {
                                                            new: settings.get_untracked(),
                                                        },
                                                    )
                                                    .unwrap();
                                                let _ = invoke("set_settings", args).await;
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
            <p>"Заполнить блок - содержимое About"</p>
        </div>
    }
}

#[component]
fn Refs() -> impl IntoView {
    let i18n = use_i18n();
    let stat: RwSignal<StatisticsState> = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let selected_ref: RwSignal<Option<String>> = use_context().expect("selected not found");
    //let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");
    let patterns = ["fail", "error"];

    //upd_stat(stat,now);
    /*Effect::new(move |_| {
        log::info!("stat2: {:?}", stat.get());
    });*/


    view! {
        <div class="refs-list grida">
            <For
                each=move || stat.get().db_list.clone()
                key=|item| item.clone()
                children=move |item: String| {
                    view! {
                        <button
                            class="rlist"
                            on:click=move |_| { selected_ref.set(Some(item.clone())) }
                        >
                            {remove_refer_ext(item.clone())}
                        </button>
                    }
                }
            />
        </div>

        <div class="grid2 stat_table">
            <ins>{t!(i18n, references.path_ref)}": "</ins>
            <span>{move || stat.get().db_path}</span>
            <ins>{t!(i18n, references.path_ref_access)}": "</ins>
            <span class:error=move || {
                patterns.iter().any(|p| stat.get().db_path_ok.to_lowercase().contains(p))
            }>{move || stat.get().db_path_ok}</span>
            <ins>{t!(i18n, references.number)}": "</ins>
            <span>{move || stat.get().db_list.len()}</span>
            <ins>{t!(i18n, references.size)}": "</ins>
            <span>{move || read_size(stat.get().db_path_size)}</span>
            <ins>{t!(i18n, references.log_path)}": "</ins>
            <span>{move || stat.get().log_path}</span>
        </div>
        <LogViewer />
    }
}

#[component]
fn Ref() -> impl IntoView {
    let selected_ref: RwSignal<Option<String>> = use_context().expect("selected not found");
    let edit_ref: RwSignal<bool> = use_context::<RwSignal<bool>>().expect("reference not found");
    //let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");
    /*Effect::new(move |_| {
        log::info!("Refs component rendered, selected: {:?}", selected_ref.get());
    });*/
    view! {
        <div class="ref">
            <h3>"Ref(): "{move || selected_ref.get().unwrap_or_default()}</h3>
        </div>
        <button on:click=move |_| {
            edit_ref.set(true)
        }>"✎ Edit " {move || selected_ref.get().unwrap_or_default()}</button>
    }
}

#[component]
fn Edit() -> impl IntoView {
    let i18n = use_i18n();
    let selected_ref: RwSignal<Option<String>> = use_context().expect("selected not found");
    let edit_ref: RwSignal<bool> = use_context::<RwSignal<bool>>().expect("edit not found");
    let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");

    let del_ref = move |name: String| {
        spawn_local(async move {
            let args = to_value(&ToBack { val: name.clone()}).unwrap();
            let js = invoke("del_ref", args).await;
            
            match from_value::<String>(js) {
                Ok(_s) => {
                    set_now(now, format!("{}: {}", tu_string!(i18n, edit.ok_del_ref), &name));
                    selected_ref.set(None::<String>);
                    edit_ref.set(false);
                    //upd_stat(stat,now);
                },
                Err(e) => {
                    set_now(now, format!("{}: {} - {}", tu_string!(i18n, edit.er_del_ref), &name, e));
                }
            }
        });
    };
    view! {
        <h3>{t!(i18n, edit.test)}</h3>

        <button on:click=move |_| {
            edit_ref.set(false)
        }>"✎ Save " {move || selected_ref.get()}</button>
        <button on:click=move |_| del_ref(selected_ref.get().unwrap_or_default())>"Del"</button>
    }
}

#[component]
fn Create() -> impl IntoView {
    use leptos::ev::SubmitEvent;
    //use web_sys::{HtmlFormElement,HtmlInputElement, FormData}; // Импорт напрямую из web_sys

    let i18n = use_i18n();
    let selected_ref: RwSignal<Option<String>> = use_context().expect("selected not found");
    let stat: RwSignal<StatisticsState> = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");
    let err_send: RwSignal<String> = RwSignal::new("".to_string());
    let active_tab: RwSignal<i32> = use_context().expect("active_tab not found");
    // mode: "empty" | "sheet" | "sqlite"
    let mode = RwSignal::new("empty".to_string());
    let form_ref = NodeRef::<leptos::html::Form>::new();
    let ex_names = ["shrinkflation", "222", "333"];

    let submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        
        let Some(form) = form_ref.get() else { return };
        
        // Создаём структуру
        let mut form_data = CreateForm { mode: mode.get(), ..Default::default() };
        
        // Собираем текстовые поля
        let elements = form.elements();
        
        for i in 0..elements.length() {
            if let Some(element) = elements.item(i)
                && let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                    match input.name().as_str() {
                        "db_name" => form_data.db_name = input.value(),
                        "csv_delim" => form_data.csv_delim = input.value(),
                        "has_header" => form_data.has_header = input.checked(),
                        _ => {}
                    }
                }
        }

        // Проверяем обязательные поля
        if form_data.db_name.is_empty() {
            log::error!("Имя базы данных обязательно");
            return;
        }
        
        // Получаем файл для чтения ВНУТРИ spawn_local
        let file_to_read = if form_data.mode != "empty" {
            let file_input = form.query_selector("input[type='file']")
                .ok()
                .flatten()
                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok());
            
            if let Some(file_input) = file_input {
                if let Some(file_list) = file_input.files() {
                    if file_list.length() > 0 {
                        file_list.item(0)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Проверка расширения файла
        if let Some(file) = &file_to_read {
            let file_name = file.name();
            let extension = get_file_extension(&file_name).to_lowercase();
            
            match form_data.mode.as_str() {
                "sheet" => {
                    let allowed = ["csv", "xls", "xlsx", "ods"];
                    if !allowed.contains(&extension.as_str()) {
                        log::error!("Недопустимый формат файла. Разрешены: csv, xls, xlsx, ods");
                        return;
                    }
                    form_data.file_extension = Some(extension);
                }
                "sqlite" => {
                    let allowed = ["sqlite", "sqlite3", "db"];
                    if !allowed.contains(&extension.as_str()) {
                        log::error!("Недопустимый формат файла. Разрешены: sqlite, sqlite3, db");
                        return;
                    }
                    form_data.file_extension = Some(extension);
                }
                _ => {}
            }
        }
        
        // Все проверки пройдены - запускаем spawn_local
        spawn_local(async move {
            // Читаем файл внутри async блока
            if let Some(file) = file_to_read {
                match read_file_as_bytes(&file).await {
                    Ok(bytes) => form_data.file_data = Some(bytes),
                    Err(e) => {
                        log::error!("Ошибка чтения файла: {}", e);
                        set_now(now, format!("{}: {} - {}", 
                            tu_string!(i18n, create.er_create), 
                            &form_data.db_name, 
                            e));
                        return;
                    }
                }
            }
            
            log::info!("📦 Отправляем: {:?}", form_data);
            
            let args = to_value(&CreateFormBack { val: form_data.clone() }).unwrap();
            let js = invoke("create", args).await;
            match from_value::<String>(js) {
                Ok(_s) => set_now(now,format!("{}: {}", tu_string!(i18n, create.ok_create), &form_data.db_name)),
                Err(e) => set_now(now,format!("{}: {} - {}", tu_string!(i18n, create.er_create), &form_data.db_name,e))
            }
        });
    };

    let create_ex = move |name:&str| {
        let form_data = CreateForm { mode: "example".to_string(), ..Default::default() };
        let name = name.to_string();
        spawn_local(async move {
            let args = to_value(&CreateFormBack { val: form_data }).unwrap();
            let js = invoke("create", args).await;
            match from_value::<String>(js) {
                Ok(_s) => {
                    set_now(now,format!("{}: {}", tu_string!(i18n, create.ok_create), &name));
                    selected_ref.set(Some("abook.refer".to_string()));
                    active_tab.set(1);
                },
                Err(e) => set_now(now,format!("{}: {} - {}", tu_string!(i18n, create.er_create), &name,e))
            }
        });
    };

    view! {
        <Show
            when=move || stat.get().db_path_ok == "Ok"
            fallback=move || {
                view! {
                    <p>{tu!(i18n, create.main_error)}":"</p>
                    <span>{move || stat.get().db_path}</span>
                    <span class="error">{move || stat.get().db_path_ok}</span>
                }
            }
        >
            <span class="err_send">{move || err_send.get()}</span>
            <form class="form_new" on:submit=submit node_ref=form_ref>
                <fieldset class="grida">
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
                </fieldset>

                <div class="block gridl">
                    <label>
                        {t!(i18n, create.fname)}
                        <span data-placement="right" data-tooltip=t_string!(i18n, create.ttp_path)>
                            "?"
                        </span>
                    </label>

                    <input type="text" name="db_name" placeholder="my_refer" required />
                </div>

                {move || {
                    match mode.get().as_str() {
                        "sheet" => {
                            view! {
                                <div class="gridl">
                                    <label for="file">
                                        {t!(i18n, create.ftable)}
                                        <span
                                            data-placement="right"
                                            data-tooltip=t_string!(i18n, create.ttp_table)
                                        >
                                            "?"
                                        </span>
                                    </label>
                                    <input
                                        id="file"
                                        type="file"
                                        name="file"
                                        accept=".csv,.xls,.xlsx,.ods"
                                        required
                                    />
                                </div>
                                <div class="gridl">
                                    <label>
                                        {t!(i18n, create.fheader)}
                                        <span
                                            data-placement="right"
                                            data-tooltip=t_string!(i18n, create.ttp_header)
                                        >
                                            "?"
                                        </span>
                                    </label>
                                    <input type="checkbox" class="" name="has_header" />
                                </div>
                            }
                                .into_any()
                        }
                        "sqlite" => {
                            view! {
                                <div class="gridl">
                                    <label for="sqlite_file">
                                        {t!(i18n, create.fsqlite)}
                                        <span
                                            data-placement="right"
                                            data-tooltip=t_string!(i18n, create.ttp_sqlite)
                                        >
                                            "?"
                                        </span>
                                    </label>
                                    <input
                                        id="sqlite_file"
                                        type="file"
                                        name="file"
                                        accept=".sqlite,.sqlite3,.db"
                                        required
                                    />
                                </div>
                            }
                                .into_any()
                        }
                        _ => view! { <div></div> }.into_any(),
                    }
                }}

                <div class="actions">
                    <button type="submit">{t!(i18n, create.title)}</button>
                </div>
            </form>
            <hr />
            <h5>{t!(i18n, create.example)}</h5>
            <p class="center">{t!(i18n, create.example_replace)}</p>
            <div class="test_create gridl">
                <button on:click=move |_| create_ex(ex_names[0])>{ex_names[0]}</button>
                <span>{t!(i18n, create.example_desc_1)}</span>
                <button on:click=move |_| create_ex(ex_names[1])>{ex_names[1]}</button>
                <span>{t!(i18n, create.example_desc_2)}</span>
                <button on:click=move |_| create_ex(ex_names[2])>{ex_names[2]}</button>
                <span>{t!(i18n, create.example_desc_3)}</span>
            </div>
        </Show>
    }
}

#[component]
fn LogViewer() -> impl IntoView {
    let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");
    let logs = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let i18n = use_i18n();

    let load_logs = move |_| {
        loading.set(true);
        
        spawn_local(async move {
            let js = invoke("get_log", JsValue::NULL).await;
            match from_value::<String>(js){
                Ok(log_content) => {
                    logs.set(Some(log_content));
                    loading.set(false);
                    set_now(now,"Logs uploaded".to_string());
                }
                Err(e) => {
                    set_now(now,format!("Failed to load logs: {}", e));
                    logs.set(None);
                    loading.set(false);
                }
            }
        });
    };
    
    let clear_logs = move |_| {
        logs.set(None);
        spawn_local(async move {
            let js = invoke("clear_log", JsValue::NULL).await;
            match from_value::<String>(js){
                Ok(_) => {
                    set_now(now,"Logs deleted".to_string());
                }
                Err(e) => {
                    set_now(now,format!("Failed to load logs: {}", e));
                }
            }
        });
    };

    view! {
        <div class="log-viewer">
            <div class="log-controls">
                <button on:click=load_logs disabled=move || loading.get() class="btn btn-primary">
                    {move || match loading.get() {
                        true => t!(i18n, references.loading).into_any(),
                        false => t!(i18n, references.show_logs).into_any(),
                    }}
                </button>
                <button on:click=clear_logs class="btn btn-secondary">
                    {t!(i18n, references.clear_logs)}
                </button>
            </div>

            <div class="log-content">
                <code>{move || logs.get().unwrap_or_default()}</code>
            </div>
        </div>
    }
}

fn read_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB * 10.0 {
        // показывать в байтах до ~10 KiB как целое
        format!("{} B", bytes)
    } else if b < KB * KB {
        // KiB с 1 знаком
        format!("{:.1} KiB", b / KB)
    } else {
        // MiB с 1 знаком (и дальше можно дополнять GiB и т.д.)
        format!("{:.1} MiB", b / (KB * KB))
    }
}

fn upd_stat(stat: RwSignal<StatisticsState>, now: RwSignal<String>) {
    spawn_local(async move {
        let js = invoke("get_stat", JsValue::NULL).await;
        match from_value::<StatisticsState>(js) {
            Ok(new_stat) => stat.set(new_stat),
            Err(e) => now.set(format!("Err: {}", e)),
        };
    });
}

fn set_now(now: RwSignal<String>, message: String){
    now.set(message);
    set_timeout(
        move || now.set("".to_string()),
        std::time::Duration::from_millis(4000)
    );
}

fn remove_refer_ext(mut s: String) -> String {
    if s.ends_with(".refer") {
        let new_len = s.len() - ".refer".len();
        s.truncate(new_len);
    }
    s
}

fn get_file_extension(filename: &str) -> String {
    filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_string()
}

async fn read_file_as_bytes(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let array_buffer_promise = file.array_buffer();
    let array_buffer = wasm_bindgen_futures::JsFuture::from(array_buffer_promise)
        .await
        .map_err(|e| format!("Ошибка чтения файла: {:?}", e))?;
    
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}
// app.rs или отдельный модуль helpers.rs
/*
    /*Effect::new(move |_| {
        log::info!("stat00: {:?}", stat.get());
        log::info!("set00: {:?}", settings.get());
        log::info!("now00: {:?}", now.get());
    });*/

fn show_success(now: RwSignal<String>, i18n: &I18nContext<Locale, I18nKeys>, key: impl Into<String>) {
    now.set(t_string!(i18n, key).to_string());
    clear_after_delay(now.clone(), 3000);
}

fn show_error(now: RwSignal<String>, i18n: &I18nContext<Locale, I18nKeys>, key: &str, details: &str) {
    now.set(format!("{}: {}", t_string!(i18n, key), details));
    clear_after_delay(now.clone(), 5000);
}

fn clear_after_delay(now: RwSignal<String>, ms: u32) {
    set_timeout(
        move || now.set("".to_string()),
        std::time::Duration::from_millis(ms as u64)
    );
}*/

// convert code 2 lang
/*
fn key_convert(r: &str) -> String {
    let i18n = use_i18n();
    match r {
        "err_test" => t_string!(i18n, err.err_test).to_string(),
        _ => t_string!(i18n, err.err_unknown).to_string(),
    }
}

                            <span
                                data-placement="right"
                                data-tooltip=t_string!(i18n, create.ttp_path)
                            >
                                "?"
                            </span>


    "crate_title": "Create",
    "edit_title": "Edit"
    let stat: RwSignal<StatisticsState> = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let selected_ref: RwSignal<String> = use_context::<RwSignal<String>>().expect("selected not found");

        spawn_local(async move {
            let commands = commands.get_untracked();
            let args = to_value(&SaveBackArgs { commands }).unwrap();
            let js = invoke("set_commands", args).await;
            let result: Result<String, String> = from_value(js).map_err(|e| format!("deserialize failed: {e}"));
            match result {
                Ok(_) => { set_status.set("Ok( Commands saved )".to_string());}
                Err(e) => set_status.set(format!("Err( Save failed: {e} )")),
            }
            let _ = invoke("request_restart", JsValue::NULL).await;
        });


*/
