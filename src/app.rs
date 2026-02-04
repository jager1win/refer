use leptos::{prelude::*};
use leptos::task::spawn_local;
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

#[derive(Serialize)]
struct SQLBack {
    new: Vec<String>,
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

#[component]
pub fn App() -> impl IntoView {
    let settings: RwSignal<AppSettings> = RwSignal::new(AppSettings {theme: "light".into(),language: "en".into()});
    let stat: RwSignal<StatisticsState> = RwSignal::new(StatisticsState::default());
    let selected_ref:RwSignal<Option<String>>  = RwSignal::new(None::<String>);
    let edit_ref: RwSignal<bool> = RwSignal::new(false);
    let active_tab: RwSignal<i32> = RwSignal::new(1);
    let now: RwSignal<String> = RwSignal::new(String::from(""));

    provide_context(settings);
    provide_context(stat);
    provide_context(selected_ref);
    provide_context(edit_ref);
    provide_context(now);

    // init settings
    spawn_local(async move {
        let js = invoke("get_settings", JsValue::NULL).await;
        match from_value::<AppSettings>(js) {
            Ok(s) => settings.set(s),
            Err(e) => {
                let m = format!("{} {}", "Err init settings:", e);
                set_now(now,m);
            }
        };
    });

    upd_stat(stat,now);

    let clean = move || {
        selected_ref.set(None::<String>);
        edit_ref.set(false);
        set_now(now,"".to_string());
    };
    
    view! {
        <I18nContextProvider>
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
                        clean();
                        upd_stat(stat, now);
                        active_tab.set(1);
                    }
                >
                    "📚 "
                    {
                        let i18n = use_i18n();
                        t!(i18n, nav.references)
                    }
                </button>
                <button
                    class="navb"
                    class:active=move || active_tab.get() == 2
                    on:click=move |_| {
                        clean();
                        upd_stat(stat, now);
                        active_tab.set(2)
                    }
                >
                    "✚"
                </button>
            </nav>
            <Now />
            <main class="main">
                {move || match active_tab.get() {
                    0 => view! { <Settings /> }.into_any(),
                    1 => view! { <ReferencesContainer /> }.into_any(),
                    2 => view! { <Create /> }.into_any(),
                    _ => view! { <div>Unknown tab</div> }.into_any(),
                }}
            </main>
        </I18nContextProvider>
    }
}

#[component]
fn Now() -> impl IntoView {
    let i18n = use_i18n();
    let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");
    let stat: RwSignal<StatisticsState> = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let patterns = ["fail", "error"];

    Effect::new(move |_| {
        log::info!("Stat : {:?}", stat.get());
        let stat_check = stat.get();
        if !stat_check.db_path_ok.is_empty(){
            match stat_check.db_path_ok{
                val if val == "Ok" => {
                    let ready = tu_string!(i18n, references.ready).to_string();
                    set_now(now, ready)
                },
                e => set_now(now, e)
            }
        }
    });

    view! {
        <div
            class="grid now"
            class:error=move || patterns.iter().any(|p| now.get().to_lowercase().contains(p))
        >
            <p class="">{move || now.get()}</p>
        </div>
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

            <div class="locale-switcher grid2">
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

            <div class="grid2">
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
    let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");
    let patterns = ["fail", "error"];
    upd_stat(stat,now);

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
            <ins>{tu!(i18n, references.path_ref)}": "</ins>
            <span>{move || stat.get().db_path}</span>
            <ins>{tu!(i18n, references.path_ref_access)}": "</ins>
            <span class:error=move || {
                patterns.iter().any(|p| stat.get().db_path_ok.to_lowercase().contains(p))
            }>{move || stat.get().db_path_ok}</span>
            <ins>{tu!(i18n, references.number)}": "</ins>
            <span>{move || stat.get().db_list.len()}</span>
            <ins>{tu!(i18n, references.size)}": "</ins>
            <span>{move || read_size(stat.get().db_path_size)}</span>
            <ins>{tu!(i18n, references.log_path)}": "</ins>
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
        let m_ok = tu_string!(i18n, ok.del_ref);
        let m_er = tu_string!(i18n, err.del_ref);
        spawn_local(async move {
            let args = to_value(&ToBack { val: name.clone() }).unwrap();
            let js = invoke("del_ref", args).await;
            
            match from_value::<String>(js) {
                Ok(_s) => {
                    // update Now
                    let m = format!("{}: {}", m_ok, name);
                    set_now(now,m);
                    // clean selected_ref
                    selected_ref.set(None::<String>);
                    // out from edit
                    edit_ref.set(false);
                    //upd_stat(stat,now);
                },
                Err(e) => {
                    let m = format!("{} '{}': {}", m_er, name, e);
                    set_now(now,m);
                }
            }
        });
    };
    view! {
        <h3>{tu!(i18n, edit.test)}</h3>

        <button on:click=move |_| {
            edit_ref.set(false)
        }>"✎ Save " {move || selected_ref.get()}</button>
        <button on:click=move |_| del_ref(selected_ref.get().unwrap_or_default())>"Del"</button>
    }
}

#[component]
fn Create() -> impl IntoView {
    use leptos::html::Input;
    use leptos::ev::SubmitEvent;
    let i18n = use_i18n();
    let stat: RwSignal<StatisticsState> = use_context::<RwSignal<StatisticsState>>().expect("stat not found");
    let now: RwSignal<String> = use_context::<RwSignal<String>>().expect("now not found");

    let input_el: NodeRef<Input> = NodeRef::new();
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let name = input_el.get().expect("...").value();
        /*spawn_local(async move {
            let args = to_value(&js_obj!("name": name)).unwrap();
            invoke("my_cmd", args).await;
        });*/
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
            <h3>{tu!(i18n, create.title)}</h3>
            <details name="sql">
                <summary>{tu!(i18n, create.empty_ref)}</summary>
                // on_submit defined below
                <form on:submit=on_submit>
                    <input type="text" value="" node_ref=input_el />
                    <input type="submit" value="" />
                </form>
            </details>
            <details name="sql">
                <summary>{tu!(i18n, create.ref_from_table)}</summary>
                <div class="form">"form2"</div>
            </details>
            <details name="sql">
                <summary>{tu!(i18n, create.ref_from_db)}</summary>
                <div class="form">"form3"</div>
            </details>
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

// app.rs или отдельный модуль helpers.rs
/*fn show_success(now: RwSignal<String>, i18n: &I18nContext<Locale, I18nKeys>, key: impl Into<String>) {
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
