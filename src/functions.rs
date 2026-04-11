use crate::{app::*, tauri_args};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wasm_bindgen::prelude::*;

pub fn validate_relative_refer_path(p: &Path) -> Result<(), ()> {
    let s = p.to_string_lossy();

    // не должен начинаться или заканчиваться на '/'
    if s.starts_with('/') || s.ends_with('/') {
        return Err(());
    }
    // запрещаем ':' и обратный слеш
    if s.contains(':') || s.contains('\\') || s.contains("//") {
        return Err(());
    }

    // каждый компонент не пустой, не "..", без управляющих символов и длина 1..=255
    for comp in s.split('/') {
        if comp.is_empty() {
            return Err(());
        }
        if comp == ".." {
            return Err(());
        }
        if comp.chars().any(|c| c.is_control()) {
            return Err(());
        }
        let len = comp.chars().count();
        if len == 0 || len > 255 {
            return Err(());
        }
    }

    Ok(())
}

pub fn read_size(bytes: u64) -> String {
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

pub fn upd_stat(stat: RwSignal<StatisticsState>, now: RwSignal<String>) {
    spawn_local(async move {
        match invoke("get_stat", &JsValue::NULL).await {
            Ok(js) => {
                let res = from_value::<StatisticsState>(js).unwrap_or_default();
                stat.set(res);
            }
            Err(js) => {
                let error_msg = from_value::<String>(js).unwrap_or_else(|_| "Unknown error".into());
                now.set(format!("Err: {}", error_msg));
            }
        };
    });
}

pub fn remove_refer_ext(p: &Path) -> String {
    let mut s = p.display().to_string();
    if s.ends_with(".refer") {
        s.truncate(s.len() - ".refer".len());
    }
    s
}

pub fn full_pb(main: PathBuf, rel: PathBuf) -> PathBuf {
    let mut p = main;
    p.push(rel);
    p.to_path_buf()
}

pub fn sort_f_keys_v(keys: Vec<String>) -> Vec<String> {
    let mut sorted = keys;
    sorted.sort_by(|a, b| {
        let num_a = a[2..].parse::<i32>().unwrap_or(0);
        let num_b = b[2..].parse::<i32>().unwrap_or(0);
        num_a.cmp(&num_b)
    });
    sorted
}

pub fn f2name_v(v: &Vec<String>, names: &HashMap<String, String>) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for f in v {
        if names.contains_key(f) {
            res.push(names[f].clone());
        }
    }
    res
}

// Получить заголовок элемента для списка/заголовка
pub fn get_item_title(record: &DataRecord, meta: &TableMeta) -> String {
    if !meta.search_config.is_empty() {
        meta.search_config
            .iter()
            .filter_map(|field| record.fields.get(field))
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    } else {
        format!("ref: {}", record.id)
    }
}

#[component]
pub fn RunOper(oper: Operation, vars: HashMap<String, f64>) -> impl IntoView {
    use exmex::Express;
    let orig = RwSignal::new(vars.clone());
    let (inner, set_inner) = signal(vars.clone());

    // Парсинг формулы
    let formula = oper.expression.clone();
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
        names
            .into_iter()
            .filter(|n| !inner.contains_key(n))
            .collect::<Vec<String>>()
    };

    let clear_inputs = move |_| {
        let filtered_names = filtered();
        set_inner.update(|map| {
            for name in filtered_names {
                map.remove(&name);
            }
        });
    };

    view! {
        <div class="oper gr grid center gap04">
            <strong class="center">
                {oper.name}" : "
                <span>
                    {move || match calculation.get() {
                        Ok(val) => view! { <span class="success">{format!("{:.*}", oper.precision as usize, val)}</span> }.into_any(),
                        Err(e) => view! { <span class="error">{e}</span> }.into_any(),
                    }}
                </span>"  " <Prec op_id=oper.id />
                {move || {
                    if !filtered().is_empty() {
                        // Кнопка очистки только если есть поля
                        view! { <button class="ml1 sm_b" on:click=clear_inputs>"🧹"</button> }
                            .into_any()
                    } else {
                        let _: () = view! { <></> };
                        ().into_any()
                    }
                }}
            </strong>

            // Динамические инпуты для неопределенных переменных
            <div class="flex_wrap3">
                <For
                    each=move || filtered()
                    key=|name| name.clone()
                    children=move |name| {
                        let n_label = name.clone();
                        let n_input = name.clone();
                        view! {
                            <div>
                                <label>{n_label}</label>
                                <input
                                    type="number"
                                    placeholder="0.0"
                                    step="any"
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev).parse::<f64>().unwrap_or(0.0);
                                        set_inner
                                            .update(|map| {
                                                map.insert(n_input.clone(), val);
                                            });
                                    }
                                />
                            // prop:value=move || inner.get().get(&name).cloned().unwrap_or(0.0)
                            </div>
                        }
                    }
                />
            </div>

            <small>{oper.description}</small>
            <small>{oper.expression}</small>
        </div>
    }
}

#[component]
pub fn DynamicCalc(
    #[prop(default = "a * (beta + 10) / Variable ".to_string())] oper: String,
    #[prop(default = HashMap::<String, f64>::new())] vars: HashMap<String, f64>,
) -> impl IntoView {
    use exmex::Express;
    let (formula, set_formula) = signal(oper);
    let (inputs, set_inputs) = signal(vars.clone());

    // Парсинг формулы
    let expr_result = Memo::new(move |_| exmex::parse::<f64>(&formula.get()));

    fn display_name(raw: &str) -> String {
        raw.trim_matches(|c| c == '{' || c == '}').to_string()
    }

    // Список переменных (убираем скобки {} для красивого вывода в списке)
    let var_names = Memo::new(move |_| {
        match expr_result.get() {
            Ok(e) => e
                .var_names()
                .iter()
                .map(|n| display_name(n)) // " {x} " -> " x "
                .collect::<Vec<_>>(),
            Err(_) => vec![],
        }
    });

    // Расчет со скобками {}
    let calculation = Memo::new(move |_| {
        match expr_result.get() {
            Ok(e) => {
                let current_inputs = inputs.get();
                let mut vals = Vec::new();

                for raw_name in e.var_names() {
                    // Извлекаем "чистое" имя, чтобы найти его в нашем словаре инпутов
                    let clean = display_name(raw_name);
                    if let Some(val) = current_inputs.get(&clean) {
                        vals.push(*val);
                    } else {
                        return Err(format!("Введите значение для {}", clean));
                    }
                }

                e.eval(&vals)
                    .map(|v| format!("{:.4}", v))
                    .map_err(|e| format!("Ошибка: {:?}", e))
            }
            Err(e) => Err(format!("Формула: {}", e)),
        }
    });

    // Функция очистки
    let clear_all = move |_| {
        //set_formula.set(String::new());
        set_inputs.update(|map| map.clear());
    };

    let filtered = move || {
        let names = var_names.get();
        let inputs = vars.clone();
        names
            .into_iter()
            .filter(|n| !inputs.contains_key(n))
            .collect::<Vec<String>>()
    };

    view! {
        <div class="oper gr grid center">
            <section>
                <label>"Формула"</label>
                <input
                    type="text"
                    prop:value=move || formula.get()
                    on:input=move |ev| set_formula.set(event_target_value(&ev))
                    attr:data-error=move || expr_result.get().is_err()
                />
                <button on:click=clear_all>"Очистить всё"</button>
            </section>

            // Панель операторов
            <section>
                {get_standard_operators()
                    .into_iter()
                    .map(|op| {
                        view! {
                            <button on:click=move |_| {
                                set_formula
                                    .update(|f| {
                                        if !f.is_empty() && !f.ends_with(' ') {
                                            f.push(' ');
                                        }
                                        f.push_str(op);
                                        f.push(' ');
                                    });
                            }>{op}</button>
                        }
                    })
                    .collect_view()}
            </section>

            // Динамические инпуты для переменных
            {
                log::debug!("{:?}",var_names.get_untracked());
                log::debug!("{:?}",inputs.get_untracked());
            }
            <section>
                <For
                    each=move || filtered()
                    key=|name| name.clone()
                    children=move |name| {
                        let n_label = name.clone();
                        let n_input = name.clone();
                        view! {
                            <div>
                                <label>{n_label}</label>
                                <input
                                    type="number"
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev).parse::<f64>().unwrap_or(0.0);
                                        set_inputs
                                            .update(|map| {
                                                map.insert(n_input.clone(), val);
                                            });
                                    }
                                    prop:value=move || inputs.get().get(&name).cloned().unwrap_or(0.0)
                                />
                            </div>
                        }
                    }
                />
            </section>
            <footer>
                {move || match calculation.get() {
                    Ok(res) => view! { <div data-status="ok">"Результат: " {res}</div> }.into_any(),
                    Err(err) => view! { <div data-status="error">{err}</div> }.into_any(),
                }}
            </footer>
        </div>
    }
}

#[component]
pub fn Prec(op_id: u32) -> impl IntoView {
    let meta = use_context::<RwSignal<Option<TableMeta>>>().expect("meta not found");
    let selected = use_context::<RwSignal<Selected>>().expect("selected not found");
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
        let pb = full_pb(stat.get_untracked().db_path, selected.get_untracked().refer.unwrap());

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

pub fn get_standard_operators() -> Vec<&'static str> {
    vec![
        "(", ")", "+", "-", "*", "/", "^", "%", // Арифметика
        "sin", "cos", "tan", "asin", "acos", // Тригонометрия
        "atan", "atan2", "sinh", "cosh", "tanh", "exp", "ln", "log10", "log2",
        "sqrt", // Логарифмы и корни
        "abs", "signum", "floor", "ceil", "round", "PI", "TAU", "E", // Константы
    ]
}

pub fn transform_fields(
    field_names: &HashMap<String, String>, field_types: &HashMap<String, String>, fields: &HashMap<String, String>,
) -> Result<HashMap<String, f64>, String> {
    // partition рулит
    let (successes, errors): (Vec<_>, Vec<_>) = fields
        .iter()
        // only number
        .filter(|(k, _)| field_types.get(*k).map(|s| s.as_ref()) == Some("number"))
        .map(|(k, v)| {
            let name = field_names.get(k).cloned().unwrap_or_else(|| k.clone());
            v.parse::<f64>().map(|val| (name.clone(), val)).map_err(|_| name)
        })
        .partition(Result::is_ok);

    if !errors.is_empty() {
        let err_names: Vec<String> = errors.into_iter().map(Result::unwrap_err).collect();
        return Err(format!("Invalid number format in fields: {}", err_names.join(", ")));
    }

    Ok(successes.into_iter().map(Result::unwrap).collect())
}

pub fn transform_fields2(
    names: &HashMap<String, String>,
    types: &HashMap<String, String>,
    values: &HashMap<String, String>,
) -> Vec<(String, String, String, String)> { 
    let mut vec: Vec<_> = names
        .iter()
        .map(|(id, name)| {
            let t = types.get(id).cloned().unwrap_or_default();
            let v = values.get(id).cloned().unwrap_or_default();
            (id.clone(), name.clone(), t, v)
        })
        .collect();

    vec.sort_by(|a, b| a.0.cmp(&b.0));
    vec
}
