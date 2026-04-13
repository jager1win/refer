use rusqlite::{
    Connection, Error as RError, Result, Row, functions::FunctionFlags, params, types::Value, types::ValueRef,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

// Представляет одну запись из таблицы data
#[derive(Debug, Serialize, Deserialize)]
pub struct DataRecord {
    pub id: u32,
    pub fields: HashMap<String, String>, // f_0, f_1 и т.д.
}

// Метаданные таблицы из таблицы meta
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct TableMeta {
    pub info: Vec<(String, String)>,          // name, desc
    pub field_names: HashMap<String, String>, // f_0 -> "Name", f_1 -> "Age"
    pub field_types: HashMap<String, String>, // f_0 -> Text, f_1 -> Number
    pub operations: Vec<Operation>,           // вычисляемые поля
    pub search_config: Vec<String>,           // настройки поиска
    pub count_data: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Operation {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub expression: String, // "f_6 * 17 / f_20"
    pub precision: u32,
}

// Основная структура для работы с базой
#[derive(Default, Debug)]
pub struct DbState {
    pub conn: Option<Connection>,
    pub current_path: Option<PathBuf>,
    pub meta: Option<TableMeta>,
}

impl DbState {
    pub fn get_conn(&mut self, path: PathBuf) -> Result<&Connection, String> {
        if self.current_path.as_ref() != Some(&path) {
            // 1. Открываем новое соединение
            let conn = Connection::open(&path).map_err(|e| e.to_string())?;

            // 2. Регистрируем функцию в локальной переменной conn (пока она мутабельна)
            // это основная фича для поиска в utf
            conn.create_scalar_function(
                "rust_lower",
                1,
                FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_UTF8,
                |ctx| {
                    let value = ctx.get_raw(0);
                    let text = match value {
                        ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                        ValueRef::Integer(i) => i.to_string(),
                        ValueRef::Real(r) => r.to_string(),
                        _ => String::new(), // Для Null или Blob возвращаем пустую строку
                    };
                    Ok(text.to_lowercase())
                },
            )
            .map_err(|e| format!("UDF Error: {}", e))?;

            // 3. Сохраняем подготовленное соединение в state
            self.conn = Some(conn);
            self.current_path = Some(path);

            /* 4. Загружаем метаданные - moved to with
            if let Some(conn_ref) = self.conn.as_ref() {
                self.meta = self.load_meta(conn_ref).ok();
            }*/
        }

        // Возвращаем иммутабельную ссылку (как и было в заголовке метода)
        Ok(self.conn.as_ref().expect("Connection must exist here"))
    }

    // теперь замыкание получает и соединение, и ссылку на meta
    pub fn with_conn<F, T>(&mut self, path: PathBuf, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection, Option<&TableMeta>) -> Result<T, String>,
    {
        self.get_conn(path)?;
        let conn = self.conn.as_ref().unwrap();

        let fresh_meta = self.load_meta(conn).map_err(|e| e.to_string())?;
        self.meta = Some(fresh_meta);

        f(conn, Some(self.meta.as_ref().unwrap()))
    }

    fn load_meta(&self, conn: &Connection) -> rusqlite::Result<TableMeta> {
        let mut stmt = conn.prepare("SELECT key, value FROM meta")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;

        let mut meta_data: HashMap<String, String> = HashMap::new();
        for row in rows {
            let (key, value) = row?;
            meta_data.insert(key, value);
        }

        //let name = meta_data.get("name").cloned().unwrap_or_default();
        //let desc = meta_data.get("desc").cloned().unwrap_or_default();

        // Парсим с обработкой ошибок, но не паникуем
        let info: Vec<(String, String)> = match meta_data.get("info") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => Vec::new(),
        };
        let field_names: HashMap<String, String> = match meta_data.get("field_names") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => HashMap::new(),
        };

        let field_types: HashMap<String, String> = match meta_data.get("field_types") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => HashMap::new(),
        };

        let operations: Vec<Operation> = match meta_data.get("operations") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => Vec::new(),
        };

        let search_config: Vec<String> = match meta_data.get("search_config") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => Vec::new(),
        };

        let count_data: u32 = conn.query_row(&format!("SELECT COUNT(*) FROM \"{}\"", "data"), [], |r| r.get(0))?;

        Ok(TableMeta {
            info,
            field_names,
            field_types,
            operations,
            search_config,
            count_data,
        })
    }

    pub fn update_meta(&mut self, path: PathBuf, new_meta: TableMeta) -> Result<(), String> {
        // Используем with_conn, чтобы гарантировать наличие соединения
        // Игнорируем meta в аргументах замыкания, так как мы ее сейчас перезапишем
        self.with_conn(path, |conn, _| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

            let params = [
                ("info", serde_json::to_value(&new_meta.info)),
                ("field_names", serde_json::to_value(&new_meta.field_names)),
                ("field_types", serde_json::to_value(&new_meta.field_types)),
                ("operations", serde_json::to_value(&new_meta.operations)),
                ("search_config", serde_json::to_value(&new_meta.search_config)),
            ];

            for (key, val) in params {
                let json_str = val.map_err(|e| e.to_string())?.to_string();
                tx.execute(
                    "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
                    [key, &json_str],
                )
                .map_err(|e| e.to_string())?;
            }

            tx.commit().map_err(|e| e.to_string())?;
            Ok(())
        })
    }
}

/*pub fn search_items(conn: &Connection, meta: &TableMeta, query: &str) -> Result<Vec<DataRecord>, String> {
    // 1. Формируем список полей для поиска
    let search_fields = if meta.search_config.is_empty() {
        meta.field_types
            .iter()
            .filter(|(_, ftype)| matches!(ftype, FieldType::Text))
            .map(|(field, _)| field.clone())
            .collect::<Vec<String>>()
    } else {
        meta.search_config.clone()
    };
    //println!("sf: {:?}",search_fields);

    if search_fields.is_empty() {
        return Err("No search fields defined.".to_string());
    }

    // 2. Строим SQL
    let conditions: Vec<String> = search_fields.iter().map(|f| format!("LOWER({}) LIKE ?1", f)).collect();

    let where_clause = if conditions.is_empty() {
        String::from("")
    } else {
        format!("WHERE {}", conditions.join(" OR "))
    };

    let sql = format!("SELECT * FROM data {} ORDER BY id ASC LIMIT 10", where_clause);

    let query_lower = query.to_lowercase();
    let query_pattern = format!("%{}%", query_lower);

    // 3. Выполняем запрос
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let records = stmt
        .query_map([query_pattern], |row| {
            row_to_record(row)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(records)
}*/

pub fn search_items(conn: &Connection, meta: &TableMeta, query: &str) -> Result<Vec<DataRecord>, String> {
    // 1. Собираем поля для поиска
    let search_fields = if meta.search_config.is_empty() {
        meta.field_types.keys().cloned().collect::<Vec<String>>()
    } else {
        meta.search_config.clone()
    };

    if search_fields.is_empty() {
        return Err("No search fields defined.".to_string());
    }

    // 2. Строим SQL. Используем нашу Rust-функцию для регистронезависимости Unicode.
    // Применяем rust_lower и к колонке, и к искомой строке.
    let conditions: Vec<String> = search_fields
        .iter()
        .map(|f| format!("rust_lower({}) LIKE rust_lower(?1)", f))
        .collect();

    let where_clause = format!("WHERE {}", conditions.join(" OR "));
    // Сразу берем 10 штук из базы — это надежно и быстро
    let sql = format!("SELECT * FROM data {} ORDER BY id ASC LIMIT 10", where_clause);

    // Экранируем спецсимволы SQLite, чтобы поиск по "_" не выдавал всё подряд
    let safe_query = query.replace('%', "\\%").replace('_', "\\_");
    let sql_pattern = format!("%{}%", safe_query);

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    // 3. Выполняем и сразу возвращаем результат
    let result = stmt
        .query_map([sql_pattern], row_to_record)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<DataRecord>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(result)
}

pub fn update_meta_field<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<()> {
    let json_value = serde_json::to_string(value).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, json_value],
    )?;

    Ok(())
}

/// action: "create" | "update" | "delete"
pub fn apply_el_action(conn: &Connection, action: &str, rec: DataRecord) -> Result<u32, String> {
    match action {
        "create" => {
            // Создаем новую запись
            conn.execute("INSERT INTO data DEFAULT VALUES", [])
                .map_err(|e| e.to_string())?;
            let new_id = conn.last_insert_rowid();

            // Обновляем каждое поле
            for (field_name, value) in &rec.fields {
                let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_name);
                conn.execute(&sql, params![value, new_id]).map_err(|e| e.to_string())?;
            }
            Ok(new_id as u32)
        }
        "update" => {
            // Обновляем каждое поле
            for (field_name, value) in &rec.fields {
                let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_name);
                conn.execute(&sql, params![value, rec.id]).map_err(|e| e.to_string())?;
            }
            Ok(rec.id)
        }
        "delete" => {
            conn.execute("DELETE FROM data WHERE id = ?1", params![rec.id])
                .map_err(|e| e.to_string())?;
            Ok(rec.id)
        }
        _ => Err("Unknown action".to_string()),
    }
}

/// Получить один элемент по ID
pub fn get_el(conn: &Connection, id: u32) -> Result<DataRecord, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM data WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    match stmt.query_row([id], row_to_record) {
        Ok(record) => Ok(record),
        Err(e) => Err(e.to_string()),
    }
}

fn row_to_record(row: &Row) -> rusqlite::Result<DataRecord> {
    let mut fields = HashMap::new();
    let col_count = row.as_ref().column_count();

    for i in 1..col_count {
        let col_name = row.as_ref().column_name(i)?;

        // 1. Получаем сырое значение (оно может быть Text, Integer, Real, Null)
        let val_raw: Value = row.get(i)?;

        // 2. Конвертируем в строку вручную
        let val_str = match val_raw {
            Value::Text(s) => s,
            Value::Integer(n) => n.to_string(),
            Value::Real(n) => n.to_string(), // Вот это исправит вашу ошибку!
            Value::Blob(b) => String::from_utf8_lossy(&b).to_string(), // На случай бинарных данных
            Value::Null => String::new(),    // Пустая строка для NULL
        };

        fields.insert(col_name.to_string(), val_str);
    }

    Ok(DataRecord {
        id: row.get(0)?,
        fields,
    })
}

/// Добавление поля в существующую базу
pub fn add_field(
    conn: &Connection, field_index: usize, display_name: Option<&str>, field_type: &str,
) -> Result<String> {
    let field_name = format!("f_{}", field_index);

    // Добавляем колонку в data
    let sql = format!("ALTER TABLE data ADD COLUMN {} TEXT", field_name);
    conn.execute(&sql, [])?;

    // Обновляем field_names в meta
    let names_json: String =
        conn.query_row("SELECT value FROM meta WHERE key = 'field_names'", [], |row| row.get(0))?;

    let mut names: serde_json::Value = serde_json::from_str(&names_json).unwrap_or(json!({}));
    if let Some(obj) = names.as_object_mut() {
        let name = display_name.unwrap_or(&format!("#{}", field_index)).to_string();
        obj.insert(field_name.clone(), json!(name));
    }

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'field_names'",
        params![names.to_string()],
    )?;

    // Обновляем field_types в meta
    let types_json: String =
        conn.query_row("SELECT value FROM meta WHERE key = 'field_types'", [], |row| row.get(0))?;

    let mut types: serde_json::Value = serde_json::from_str(&types_json).unwrap_or(json!({}));
    if let Some(obj) = types.as_object_mut() {
        obj.insert(field_name.clone(), json!(field_type));
    }

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'field_types'",
        params![types.to_string()],
    )?;

    Ok(field_name)
}

// Добавление операции
/*pub fn add_operation(conn: &Connection, name: &str, expression: &str, description: Option<&str>) -> Result<()> {
    let ops_json: String = conn.query_row("SELECT value FROM meta WHERE key = 'operations'", [], |row| row.get(0))?;

    let mut ops: serde_json::Value = serde_json::from_str(&ops_json).unwrap_or(json!([]));

    if let Some(arr) = ops.as_array_mut() {
        arr.push(json!({
            "name": name,
            "expression": expression,
            "description": description.unwrap_or(""),
            "created_at": chrono::Local::now().to_rfc3339(),
        }));
    }

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'operations'",
        params![ops.to_string()],
    )?;

    Ok(())
}*/
// Добавление операции
pub fn add_operation(
    conn: &Connection, name: &str, expression: &str, description: &str, precision: u32,
) -> Result<(), String> {
    let ops_json: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'operations'", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let mut ops: serde_json::Value = serde_json::from_str(&ops_json).unwrap_or(json!([]));

    if let Some(arr) = ops.as_array_mut() {
        // Вычисляем новый id
        let new_id = arr
            .iter()
            .map(|op| op["id"].as_u64().unwrap_or(0) as u32)
            .max()
            .unwrap_or(0)
            + 1;

        arr.push(json!({
            "id": new_id,
            "name": name,
            "expression": expression,
            "description": description,
            "precision": precision,
        }));
    }

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'operations'",
        params![ops.to_string()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// Создание пустой базы
pub fn create_empty_database(path: &PathBuf) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    // Таблица data - только ID
    conn.execute(
        "CREATE TABLE data (
            id INTEGER PRIMARY KEY AUTOINCREMENT
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Таблица meta - вся информация в формате ключ-значение
    conn.execute(
        "CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    let info = serde_json::to_string(&vec![
        ("name".to_string(), "".to_string()),
        ("desc".to_string(), "".to_string()),
    ]).unwrap();

    // Базовая meta для пустой базы
    let now = chrono::Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO meta (key, value) VALUES 
         ('info', ?1),
         ('field_names', ?2),
         ('field_types', ?3),
         ('operations', ?4),
         ('search_config', ?5),
         ('created_at', ?6)",
        params![
            info, // info - пустое, юзер сам заполнит
            "{}", // field_names - пустой JSON объект
            "{}", // field_types - пустой JSON объект
            "[]", // operations - пустой JSON массив
            "{}", // fields to search
            &now,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ==================== Create demo ====================
// Импорт CSV данных (общая функция для демо)
pub fn import_csv(conn: &Connection, csv_data: &str, has_header: bool) -> Result<Vec<String>> {
    let mut lines = csv_data.lines();
    let mut field_names = Vec::new();

    // Обработка заголовка
    let headers: Vec<&str> = if has_header {
        if let Some(header_line) = lines.next() {
            header_line.split(',').collect()
        } else {
            Vec::new()
        }
    } else {
        // Если нет заголовка, создаем заглушки
        let first_line = lines.clone().next().unwrap_or("");
        let col_count = first_line.split(',').count();
        (0..col_count)
            .map(|i| Box::leak(format!("Field {}", i + 1).into_boxed_str()) as &str)
            .collect()
    };

    // Создаем поля
    for (i, header) in headers.iter().enumerate() {
        // Пытаемся определить тип поля по первому значению
        let first_line = lines.clone().next().unwrap_or("");
        let first_value = first_line.split(',').nth(i).unwrap_or("");
        let field_type = if first_value.parse::<f64>().is_ok() {
            "number"
        } else {
            "text"
        };

        let field_name = add_field(conn, i, Some(header), field_type)?;
        field_names.push(field_name);
    }

    // Импортируем данные
    for line in lines {
        let values: Vec<&str> = line.split(',').collect();

        conn.execute("INSERT INTO data DEFAULT VALUES", [])?;
        let record_id = conn.last_insert_rowid();

        for (i, value) in values.iter().enumerate() {
            if i < field_names.len() {
                let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_names[i]);
                conn.execute(&sql, params![value, record_id])?;
            }
        }
    }

    Ok(field_names)
}

// Основная функция создания демо-базы. Ключи на фронте: Create()-> let create_example
pub fn create_example_refers(name: &str, path: &PathBuf) -> Result<(), String> {
    match name {
        "example/ballistics.refer" => create_ballistics_refer(path),
        "example/deposit.refer" => create_deposit_refer(path),
        "example/oscillator.refer" => create_oscillator_refer(path),
        "example/geometry.refer" => create_geometry_refer(path),
        "example/shrinkflation.refer" => create_shrinkflation_refer(path),
        "example/dilution.refer" => create_dilution_refer(path),
        _ => Err(RError::SqliteFailure(rusqlite::ffi::Error::new(1), Some("Unknown file ext".to_string())).to_string()),
    }
}

pub fn create_ballistics_refer(path: &PathBuf) -> Result<(), String> {
    // Данные для баллистики
    const BALLISTICS_CSV: &str = "Caliber,Caliber(in),Weight(g),Velocity(m/s),BC(G1),Area(cm²)
7.62x39,0.312,7.9,720,0.280,0.48
5.45x39,0.220,3.4,880,0.347,0.23
.308 Winchester,0.308,11.3,800,0.475,0.48
.223 Remington,0.224,4.0,930,0.304,0.25
9x19 Parabellum,0.355,8.0,360,0.150,0.12
12x70 Slug,0.729,28.0,480,0.210,2.15
.338 Lapua Magnum,0.338,16.2,900,0.648,0.57
6.5 Creedmoor,0.264,8.9,860,0.520,0.34
.300 Winchester Magnum,0.308,11.7,880,0.590,0.42
7.62x54R,0.312,9.6,830,0.420,0.48";

    // Создаем пустую базу
    create_empty_database(path)?;

    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    // Обновляем name и desc
    /*let info = serde_json::to_string(&TableInfo {
        name: "Ballistics Data".to_string(),
        desc: "Ballistic calculator demo data".to_string(),
    })
    .unwrap();

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'info'", params![info])
        .map_err(|e| e.to_string())?;*/

    let info: Vec<(String, String)> = vec![
        ("name".to_string(), "Ballistics Data".to_string()),
        ("desc".to_string(), "Ballistic calculator demo data".to_string()),
    ];

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'info'",
        params![serde_json::to_string(&info).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    // add search field
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    import_csv(&conn, BALLISTICS_CSV, true).map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Energy (J)",
        "{Weight(g)} * {Velocity(m/s)} * {Velocity(m/s)} / 2000",
        "Kinetic energy in Joules",
        2,
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Sectional Density",
        "({Weight(g)} * 15.4324) / ({Caliber(in)} * {Caliber(in)})",
        "Sectional Density in lb/in² (classic)",
        2,
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Vertical Drop (m)",
        "((9.81 * ({distance} / {Velocity(m/s)}) * ({distance} / {Velocity(m/s)})) / 2)",
        "Bullet drop in meters due to gravity",
        2,
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Wind Drift (m)",
        "{wind speed} * ({distance} / {Velocity(m/s)}) * (1 / {BC(G1)})",
        "Wind drift in meters (simplified with BC factor)",
        2,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create_deposit_refer(path: &PathBuf) -> Result<(), String> {
    const DEPOSIT_CSV: &str = "Operation
Deposit";

    create_empty_database(path)?;
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'name'", params!["Deposit"])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'desc'",
        params!["Calculate compound interest growth"],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    import_csv(&conn, DEPOSIT_CSV, true).map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Future Value",
        "Principal * (1 + Rate / 100) ^ Years",
        "Projected amount after compound interest",
        2,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_oscillator_refer(path: &PathBuf) -> Result<(), String> {
    // Добавлена колонка Time Hint - подсказка какое время вводить
    const OSCILLATOR_CSV: &str = "Wave,Frequency Hz,Amplitude,Time Hint (s)
Audio A4,440,1.0,0.001-0.01
Radio FM,100000000,0.5,0.00000001
WiFi 2.4GHz,2400000000,0.3,0.0000000001
WiFi 5GHz,5000000000,0.3,0.0000000001
Green Light,545000000000000,1.0,0.000000000000001";

    create_empty_database(path)?;
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'name'", params!["Oscillator"])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'desc'",
        params!["Wave value at time t - use Time Hint for reference"],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    import_csv(&conn, OSCILLATOR_CSV, true).map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Instant Value",
        "Amplitude * sin(2 * PI * {Frequency Hz} * time)",
        "Wave displacement at time t (seconds)",
        16,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_geometry_refer(path: &PathBuf) -> Result<(), String> {
    const GEOMETRY_CSV: &str = "Calculation
Circle/Sphere";

    create_empty_database(path)?;
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'name'", params!["Geometry"])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'desc'",
        params!["Circle and sphere measurements - enter your radius"],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    import_csv(&conn, GEOMETRY_CSV, true).map_err(|e| e.to_string())?;

    // Единицы измерения в названии операции - результат сразу понятен
    add_operation(
        &conn,
        "Area (cm²/in²/etc)",
        "PI * Radius ^ 2",
        "Area of circle in square units",
        2,
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Circumference (cm/in/etc)",
        "2 * PI * Radius",
        "Length of circle boundary in same units",
        2,
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Volume (cm³/in³/etc)",
        "4 / 3 * PI * Radius ^ 3",
        "Volume of sphere in cubic units",
        2,
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Surface Area (cm²/in²/etc)",
        "4 * PI * Radius ^ 2",
        "Surface area of sphere in square units",
        2,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_shrinkflation_refer(path: &PathBuf) -> Result<(), String> {
    // Пресеты систем: множитель для перевода к базовой единице
    // Работает с любыми валютами и единицами измерения
    const SHRINK_CSV: &str = "Unit System,Multiplier
Per 1000 (kg/L),1000
Per 16 (oz),16
Per 1 (base unit),1";

    create_empty_database(path)?;
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'name'",
        params!["Shrinkflation"],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'desc'",
        params!["Compare prices per unit weight/volume"],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    import_csv(&conn, SHRINK_CSV, true).map_err(|e| e.to_string())?;

    // Универсальная формула: работает с любыми единицами
    // Единицы в названии через черту — результат сразу понятен
    add_operation(
        &conn,
        "Price per Unit (per kg/L/oz/lb/etc)",
        "(Price / Weight) * Multiplier",
        "Normalized price: enter price and weight/volume, get price per base unit",
        2,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_dilution_refer(path: &PathBuf) -> Result<(), String> {
    const DILUTION_CSV: &str = "Calculator
Dilution";

    create_empty_database(path)?;
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'name'", params!["Dilution"])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'desc'",
        params!["Calculate solution mixing ratios"],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    import_csv(&conn, DILUTION_CSV, true).map_err(|e| e.to_string())?;

    // Formula 1: V_conc = (need% * need_vol) / have%
    add_operation(
        &conn,
        "Concentrate Volume (ml/L/etc)",
        "(TargetConc * TargetVol) / StockConc",
        "Volume of stock solution to use",
        2,
    )
    .map_err(|e| e.to_string())?;

    // Formula 2: V_water = total - concentrate
    add_operation(
        &conn,
        "Water Volume (ml/L/etc)",
        "TargetVol - ((TargetConc * TargetVol) / StockConc)",
        "Volume of water/solvent to add",
        2,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
