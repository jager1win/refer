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
    pub db_count: u32,
    pub db_list: Vec<String>,
}

#[component]
pub fn App() -> impl IntoView {
    leptos_meta::provide_meta_context();
    let (settings, set_settings) = signal(AppSettings {
        theme: "light".into(),
        language: "en".into(),
    });
    let status = RwSignal::new(String::from(""));

    let active_tab = RwSignal::new(1);

    // init settings
    spawn_local(async move {
        let js = invoke("get_settings", JsValue::NULL).await;
        match from_value::<AppSettings>(js) {
            Ok(s) => set_settings.set(s),
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
                    on:click={move |_| active_tab.set(1)}
                >{let i18n = use_i18n();t!(i18n, nav.references)} </button>
                <button
                    class:active={move || active_tab.get() == 2}
                    on:click={move |_| active_tab.set(2)}
                >"📥"</button>
            </nav>
            <div class="hidden">{move || status.get()}</div>
            <main class="container">
                <div class="tab-content" class:active={move || active_tab.get() == 0}>
                    <Settings getter=settings setter=set_settings/>
                </div>
                <div class="tab-content" class:active={move || active_tab.get() == 1}>
                    <References />
                </div>
                <div class="tab-content" class:active={move || active_tab.get() == 2}>
                    <Import />
                </div>
            </main>
        </I18nContextProvider>
    }
}

#[component]
fn Settings(getter: ReadSignal<AppSettings>, setter: WriteSignal<AppSettings>) -> impl IntoView {
    let i18n = use_i18n();
    let all: &[Locale] = Locale::get_all();

    //let current = move || i18n.get_locale();
    let toggle_theme = move |_| {
        setter.update(|current| {
            if current.theme == "light" {
                current.theme = "dark".to_string();
            } else {
                current.theme = "light".to_string();
            }
            spawn_local(async move {
                let args = to_value(&SettingsBack { new: getter.get() }).unwrap();
                let _ = invoke("set_settings", args).await;
            });
        });
    };

    Effect::new(move |_|{
        let lang_code = getter.get().language;
        if let Some(&loc) = all.iter().find(|l| l.to_string() == lang_code)
            .or_else(|| all.iter().find(|l| l.to_string() == "en"))
            .or(all.first())
        {
            i18n.set_locale(loc);
        }
    });

    Effect::new(move |_| {
        let theme_value = getter.get().theme;
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
                        let is_active = move || getter.get().language == loc.to_string();
                        view! {
                            <button
                                class=move || if is_active() { "locale-btn active".to_string() } else { "locale-btn".to_string() }
                                on:click=move |_| {
                                    if !is_active(){
                                        spawn_local(async move {
                                            i18n.set_locale(loc);
                                            setter.update(|current|{
                                                current.language = loc.as_str().to_string();
                                            });
                                            let args = to_value(&SettingsBack { new: getter.get() }).unwrap();
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
                    {move || match getter.get().theme.as_str() {
                        "light" => "🌙",
                        "dark" => "🌞",
                        _ => "🌞",
                    }}
                </button>
            </p>
        }
}

#[component]
fn References() -> impl IntoView {
    let i18n = use_i18n();
    let stat = RwSignal::new(StatisticsState { db_path: String::new(), db_path_size: 0, db_count: 0, db_list:Vec::new() });
    let status = RwSignal::new(String::from(""));

    spawn_local(async move {
        let js = invoke("get_stat", JsValue::NULL).await;
        match from_value::<StatisticsState>(js) {
            Ok(s) => stat.set(s),
            Err(e) => status.set(format!("deserialize failed: {}", e)),
        };
    });

    view! {
            <h3>Доступные базы</h3>

            <h3>Статистика:</h3>
            <ul>
                <li>"Папка: " {move || stat.get().db_path}</li>
                <li>"Размер: " {move || stat.get().db_path_size}</li>
                <li>"Количество баз: " {move || stat.get().db_count}</li>
                <li>"Список: " {move || stat.get().db_list}</li>
            </ul>

    }
}

#[component]
fn Import() -> impl IntoView {
    let i18n = use_i18n();

    view! {
            <h2>{t!(i18n, import.title)}</h2>

    }
}
