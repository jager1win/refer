use crate::commands::CreateForm;
use rusqlite::{Connection, Error, Result, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::error;

// Представляет одну запись из таблицы data
#[derive(Debug, Serialize, Deserialize)]
pub struct DataRecord {
    pub id: i64,
    pub fields: HashMap<String, FieldValue>, // field_0, field_1 и т.д.
}

// Разные типы полей для гибкой обработки
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Null,
}

// Метаданные таблицы из таблицы meta
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct TableMeta {
    pub field_names: HashMap<String, String>, // field_0 -> "Name", field_1 -> "Age"
    pub field_types: HashMap<String, FieldType>, // field_0 -> Text, field_1 -> Number
    pub operations: Vec<Operation>,           // вычисляемые поля
    pub search_config: SearchConfig,          // настройки поиска
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Operation {
    pub name: String, // "Total Price"
    pub description: String,
    pub expression: String, // "field_6 * 17 / field_20"
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct SearchConfig {
    pub fields: Vec<String>, // какие field_* участвуют в поиске
    pub case_sensitive: bool,
}

// Основная структура для работы с базой
#[derive(Default, Debug)]
pub struct DbState {
    pub conn: Option<Connection>,
    pub current_path: Option<PathBuf>,
    pub meta: Option<TableMeta>,
}

impl DbState {
    pub fn get_conn(&mut self, path: PathBuf) -> Result<&mut Connection, String> {
        if self.current_path.as_ref() != Some(&path) {
            // Присваивание Some автоматически дропает предыдущий Connection (закрывает файл)
            self.conn = Some(Connection::open(&path).map_err(|e| e.to_string())?);
            self.current_path = Some(path);
            // безопасно взять ссылку на только что присвоенный Connection
            if let Some(ref conn_ref) = self.conn {
                // пытаемся загрузить meta; при ошибке парсинга/SQL — вернуть Err
                let meta = self.load_meta(conn_ref).map_err(|e| e.to_string())?;
                self.meta = Some(meta);
            } else {
                // если meta невалидный - то None
                self.meta = None;
            }
        }
        Ok(self.conn.as_mut().unwrap())
    }

    pub fn with_conn<F, T>(&mut self, path: PathBuf, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, String>,
    {
        let conn = self.get_conn(path)?;
        f(conn)
    }

    fn load_meta(&self, conn: &Connection) -> rusqlite::Result<TableMeta> {
        let mut stmt = conn.prepare("SELECT key, value FROM meta")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;

        let mut meta_data: HashMap<String, String> = HashMap::new();
        for row in rows {
            let (key, value) = row?;
            meta_data.insert(key, value);
        }

        // Парсим с обработкой ошибок, но не паникуем
        let field_names: HashMap<String, String> = match meta_data.get("field_names") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => HashMap::new(),
        };

        let field_types: HashMap<String, FieldType> = match meta_data.get("field_types") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => HashMap::new(),
        };

        let operations: Vec<Operation> = match meta_data.get("operations") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => Vec::new(),
        };

        let search_config: SearchConfig = match meta_data.get("search_config") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => SearchConfig::default(),
        };

        Ok(TableMeta {
            field_names,
            field_types,
            operations,
            search_config,
        })
    }
}

pub fn search_items(conn: &Connection, meta: &TableMeta, query: &str) -> Result<Vec<DataRecord>, String> {
    // 1. Формируем список полей для поиска
    let search_fields = if meta.search_config.fields.is_empty() {
        meta.field_types
            .iter()
            .filter(|(_, ftype)| matches!(ftype, FieldType::Text))
            .map(|(field, _)| field.clone())
            .collect::<Vec<String>>()
    } else {
        meta.search_config.fields.clone()
    };

    if search_fields.is_empty() {
        return Ok(vec![]);
    }

    // 2. Строим SQL
    let conditions: Vec<String> = search_fields.iter().map(|f| format!("{} LIKE ?1", f)).collect();

    let sql = format!("SELECT * FROM data WHERE {} LIMIT 10", conditions.join(" OR "));

    let query_pattern = format!("%{}%", query);

    // 3. Выполняем запрос
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let records = stmt
        .query_map([query_pattern], |row| {
            // row_to_record теперь должна быть либо статической функцией,
            // либо методом DataRecord, принимающим &Row и &TableMeta
            row_to_record(row)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(records)
}

// Получить один элемент по ID
pub fn get_item(conn: &Connection, id: i64) -> Result<Option<DataRecord>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM data WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    match stmt.query_row([id], row_to_record) {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_db_stats(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut stats = HashMap::new();
    println!("get_db_stats");
    // 1. Получаем список таблиц
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")?;
    let tables: Vec<String> = stmt.query_map([], |row| row.get(0))?.collect::<Result<_>>()?;

    let mut table_names = Vec::new();
    let mut counts = Vec::new();
    let mut all_fields = Vec::new();

    for table in tables {
        // Имена таблиц
        table_names.push(table.clone());

        // Количество записей
        let count: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM \"{}\"", table), [], |r| r.get(0))?;
        counts.push(count.to_string());

        // Поля (собираем в одну строку "col1, col2" или можно пушить по отдельности)
        let mut col_stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table))?;
        let fields: Vec<String> = col_stmt.query_map([], |row| row.get(1))?.collect::<Result<_>>()?;
        all_fields.push(fields.join(", ")); // Объединяем поля таблицы в одну строку
    }

    stats.insert("tables".to_string(), table_names);
    stats.insert("count".to_string(), counts);
    stats.insert("fields".to_string(), all_fields);

    Ok(stats)
}

fn row_to_record(row: &Row) -> Result<DataRecord> {
    let id = row.get(0)?;
    let mut fields = HashMap::new();

    // Получаем все колонки, начиная с 1 (после id)
    let column_count = row.as_ref().column_count();
    for i in 1..column_count {
        let column_name = row.as_ref().column_name(i)?.to_string();

        // Пробуем получить как число, если не получается - как текст
        if let Ok(num) = row.get::<_, f64>(i) {
            fields.insert(column_name, FieldValue::Number(num));
        } else if let Ok(text) = row.get::<_, String>(i) {
            fields.insert(column_name, FieldValue::Text(text));
        } else {
            fields.insert(column_name, FieldValue::Null);
        }
    }
    println!("rtr: {}, {:?}", &id, &fields);
    Ok(DataRecord { id, fields })
}

/*pub fn get_all_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare("SELECT * FROM items ORDER BY name")?;
    let items = stmt.query_map([], |row| {
        Ok(Item {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
        })
    })?;

    let mut result = Vec::new();
    for item in items {
        result.push(item?);
    }
    Ok(result)
}*/
// mutex db conn
/*n open_db(path: String, db_state: tauri::State<'_, DbState>) -> Result<(), String> {
    use rusqlite::Connection;
    let conn = Connection::open(&path)
        .map_err(|e| format!("Ошибка открытия БД: {}", e))?;

    // 2. Обновляем DbState (активное соединение)
    let mut db_lock = db_state.0.lock().unwrap();
    *db_lock = Some(conn);

    Ok(())
}*/

// ==================== Базовые операции с БД ====================

// Создание пустой базы
pub fn create_empty_database(path: &PathBuf) -> Result<()> {
    let conn = Connection::open(path)?;

    // Таблица data - только ID
    conn.execute(
        "CREATE TABLE data (
            id INTEGER PRIMARY KEY AUTOINCREMENT
        )",
        [],
    )?;

    // Таблица meta - вся информация в формате ключ-значение
    conn.execute(
        "CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    // Базовая meta для пустой базы
    let now = chrono::Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO meta (key, value) VALUES 
         ('table_name', ?1),
         ('table_description', ?2),
         ('field_names', ?3),
         ('field_types', ?4),
         ('operations', ?5),
         ('created_at', ?6)",
        params![
            "",   // table_name - пустое, юзер сам заполнит
            "",   // table_description - пустое
            "{}", // field_names - пустой JSON объект
            "{}", // field_types - пустой JSON объект
            "[]", // operations - пустой JSON массив
            &now,
        ],
    )?;

    Ok(())
}

pub fn create_from_file(path: &PathBuf, val: &CreateForm) -> Result<()> {
    Ok(())
}

// Добавление поля в существующую базу
pub fn add_field(
    conn: &Connection, field_index: usize, display_name: Option<&str>, field_type: &str,
) -> Result<String> {
    let field_name = format!("f_{}", field_index);

    // Добавляем колонку в data
    let sql = format!(
        "ALTER TABLE data ADD COLUMN {} {}",
        field_name,
        match field_type {
            "number" | "real" => "REAL",
            "integer" => "INTEGER",
            _ => "TEXT",
        }
    );
    conn.execute(&sql, [])?;

    // Обновляем field_names в meta
    let names_json: String =
        conn.query_row("SELECT value FROM meta WHERE key = 'field_names'", [], |row| row.get(0))?;

    let mut names: serde_json::Value = serde_json::from_str(&names_json).unwrap_or(json!({}));
    if let Some(obj) = names.as_object_mut() {
        let name = display_name.unwrap_or(&format!("Field {}", field_index)).to_string();
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

// Импорт CSV данных (общая функция)
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
            .map(|i| Box::leak(format!("Field_{}", i + 1).into_boxed_str()) as &str)
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

// Добавление операции
pub fn add_operation(conn: &Connection, name: &str, expression: &str, description: Option<&str>) -> Result<()> {
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
}

// ==================== Создание демо-баз ====================

// Данные для баллистики
const BALLISTICS_CSV: &str = "caliber,bullet_mass_g,muzzle_velocity,bc_g1,cross_section_cm2
.308 Winchester,11.3,800,0.475,0.48
.338 Lapua Mag,16.2,900,0.648,0.57
7.62x54R LPS,9.6,830,0.420,0.48
5.45x39 7N6,3.4,900,0.347,0.23
.223 Remington,4.0,930,0.304,0.25
6.5 Creedmoor,8.9,860,0.520,0.34
.300 Win Mag,11.7,880,0.590,0.42
9x19 Parabellum,8.0,360,0.150,0.12
.45 ACP,15.0,260,0.180,0.16
12 gauge slug,28.0,480,0.210,2.15";

pub fn create_ballistics_database(path: &PathBuf) -> Result<()> {
    // Создаем пустую базу
    create_empty_database(path)?;

    let conn = Connection::open(path)?;

    // Обновляем table_name и table_description
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'table_name'",
        params!["Ballistics Data"],
    )?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'table_description'",
        params!["Ballistic trajectory calculator demo data"],
    )?;

    // Импортируем CSV с заголовком
    import_csv(&conn, BALLISTICS_CSV, true)?;

    // Добавляем операции
    add_operation(
        &conn,
        "Energy (J)",
        "f_1 * f_2 * f_2 / 2000", // bullet_mass_g * velocity^2 / 2000
        Some("Kinetic energy in Joules"),
    )?;

    add_operation(
        &conn,
        "Sectional Density",
        "f_1 / (f_4 * 1000)", // mass / cross_section_cm2 * 1000
        Some("Bullet mass / cross-sectional area"),
    )?;

    Ok(())
}

// Заглушки для будущих демо-баз
pub fn create_recipes_database(path: &PathBuf) -> Result<()> {
    // TODO: реализовать
    Ok(())
}

pub fn create_inventory_database(path: &PathBuf) -> Result<()> {
    // TODO: реализовать
    Ok(())
}

// Основная функция создания демо-базы по имени
pub fn create_example_database(name: &str, path: &PathBuf) -> Result<()> {
    match name {
        "example/ballistics.refer" => create_ballistics_database(path),
        "recipes" => create_recipes_database(path),
        "inventory" => create_inventory_database(path),
        _ => Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some(format!("Unknown demo database: {}", name)),
        )),
    }
}
