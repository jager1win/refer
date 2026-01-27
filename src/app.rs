use leptos::task::spawn_local;
use leptos::{prelude::*};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

include!(concat!(env!("OUT_DIR"), "/i18n/mod.rs"));
use i18n::*;

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

#[derive( Debug, Clone, Serialize, Deserialize)]
struct StatisticsState {
    pub db_path: String,
    pub db_path_size: u64,
    pub db_list: Vec<String>,
    pub log_path: String,
    pub errors: Vec<String>
}

#[component]
pub fn App() -> impl IntoView {
    leptos_meta::provide_meta_context();
    let settings = RwSignal::new(AppSettings {theme: "light".into(),language: "en".into()});
    let stat = RwSignal::new(StatisticsState { db_path: String::from(""), db_path_size: 0, db_list:Vec::new(), log_path: String::from(""), errors:Vec::new()});
    let status = RwSignal::new(String::from(""));
    let selected_ref = RwSignal::new(String::from(""));
    let active_tab = RwSignal::new(1);

    // init settings
    let upd_settings:() = spawn_local(async move {
        let js = invoke("get_settings", JsValue::NULL).await;
        match from_value::<AppSettings>(js) {
            Ok(s) => settings.set(s),
            Err(e) => status.set(format!("deserialize failed: {}", e)),
        };
    });
    upd_settings;

    // init statistics
    spawn_local(async move {
        let js = invoke("get_stat", JsValue::NULL).await;
        match from_value::<StatisticsState>(js) {
            Ok(s) => stat.set(s),
            Err(e) => status.set(format!("deserialize failed: {}", e)),
        };
    });

    view! {
        <I18nContextProvider>
            <nav class="top-nav">
                <button
                    class:active={move || active_tab.get() == 0}
                    on:click={move |_| active_tab.set(0)}
                >"⚙"</button>
                <button
                    class:active={move || active_tab.get() == 1}
                    on:click={move |_| {active_tab.set(1);selected_ref.set("".to_string())}}
                >{let i18n = use_i18n();t!(i18n, nav.references)} {move || selected_ref.get()}</button>
                <button
                    class:active={move || active_tab.get() == 2}
                    on:click={move |_| active_tab.set(2)}
                >"✎✚"</button>
            </nav>
            <div class="hidden">{move || status.get()}</div>
            <main class="container">
                <div class="tab-content" class:active={move || active_tab.get() == 0}>
                    <Settings settings=settings/>
                </div>
                <div class="tab-content" class:active={move || active_tab.get() == 1}>
                    <Suspense fallback=move || view! { <p>{let i18n = use_i18n();t!(i18n, references.loading)}</p> } >
                        {move || {
                            let select = selected_ref.get().is_empty();
                            if select{
                                view!{<Refs stat=stat selected=selected_ref />}.into_any()
                            }else{
                                view!{<Ref selected=selected_ref />}.into_any()
                        }}}
                    </Suspense>
                </div>
                <div class="tab-content" class:active={move || active_tab.get() == 2}>
                    <Edit stat=stat/>
                </div>
            </main>
        </I18nContextProvider>
    }
}

#[component]
fn Settings(settings: RwSignal<AppSettings>) -> impl IntoView {
    let i18n = use_i18n();
    let all: &[Locale] = Locale::get_all();

    //let current = move || i18n.get_locale();
    let toggle_theme = move |_| {
        settings.update(|current| {
            if current.theme == "light" {
                current.theme = "dark".to_string();
            } else {
                current.theme = "light".to_string();
            }
            spawn_local(async move {
                let args = to_value(&SettingsBack { new: settings.get_untracked() }).unwrap();
                let _ = invoke("set_settings", args).await;
            });
        });
    };

    Effect::new(move |_|{
        let lang_code = settings.get().language;
        if let Some(&loc) = all.iter().find(|l| l.to_string() == lang_code)
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
        html_element.set_attribute("data-theme", &theme_value).unwrap();
    });

    //log::debug!("lang: {:?}", &all);
    //log::debug!("lang: {:?}", &current);
    view! {
            <p>
                <h3>{t!(i18n, settings.title)}</h3>
                <div class="locale-switcher">
                    {all.iter().map(move |&loc| {
                        let code = loc.as_str();
                        let is_active = move || settings.get().language == loc.to_string();
                        view! {
                            <button
                                class=move || if is_active() { "locale-btn active".to_string() } else { "locale-btn".to_string() }
                                on:click=move |_| {
                                    if !is_active(){
                                        spawn_local(async move {
                                            i18n.set_locale(loc);
                                            settings.update(|current|{
                                                current.language = loc.as_str().to_string();
                                            });
                                            let args = to_value(&SettingsBack { new: settings.get_untracked() }).unwrap();
                                            let _ = invoke("set_settings", args).await;
                                        });
                                    }
                                }
                            >
                                {code}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </p>
            <p>
                <h3>{t!(i18n, theme.title)}</h3>
                <button on:click=toggle_theme class="theme-switcher" >
                    {move || match settings.get().theme.as_str() {
                        "light" => "🌙",
                        "dark" => "🌞",
                        _ => "🌞",
                    }}
                </button>
            </p>
            <p>"Заполнить блок - содержимое About"</p>
        }
}

#[component]
fn Refs(stat: RwSignal<StatisticsState>, selected:RwSignal<String>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
            <h3>Доступные справочники:</h3>
            <ul>
                <For
                    each=move || stat.get().db_list.clone()
                    key=|item: &String| item.clone()
                    children=move |item: String| view! { <li><button on:click=move |_| selected.set(item.clone()) >{item.clone()}</button></li> }
                />
            </ul>

            <h5>Статистика:</h5>
            <ul>
                <li>"Папка баз: " {move || stat.get().db_path}</li>
                <li>"Размер: " {move || read_size(stat.get().db_path_size)}</li>
                <li>"Количество баз: " {move || stat.get().db_list.len()}</li>
                <li>"Папка логов: " {move || stat.get().log_path}</li>
                <li>
                    <ul>
                        <For
                            each=move || stat.get().errors.clone()
                            key=|item: &String| item.clone()
                            children=move |item: String| view! { <li>{item.clone()}</li> }
                        />
                    </ul>
                </li>
            </ul>

    }
}

#[component]
fn Ref(selected:RwSignal<String>) -> impl IntoView {
    view!{
        <div class="ref">
            <h3>"ref"{move || selected.get()}</h3>
        </div>
    }
}

#[component]
fn Edit(stat:RwSignal<StatisticsState>) -> impl IntoView {
    let i18n = use_i18n();


    view! {
            <h3>{t!(i18n, edit.create_title)}</h3>
            <ul>
                <li><button>Создать пустой справочник</button></li>
                <li><button>Создать справочник из таблицы(csv, excel, ods)</button></li>
                <li><button>Создать справочник из sqlite</button></li>
            </ul>

            <h3>{t!(i18n, edit.edit_title)}</h3>
            <ul>
                <For
                    each=move || stat.get().db_list.clone()
                    key=|item: &String| item.clone()
                    children=move |item: String| view! { <li><button /*on:click=move |_| selected.set(item.clone())*/ >{item.clone()}</button></li> }
                />
            </ul>

    }
}

#[component]
fn CreateRef(){

}

#[component]
fn EditRef(){

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

/*
    "crate_title": "Create",
    "edit_title": "Edit"

*/