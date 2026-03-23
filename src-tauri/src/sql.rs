use crate::commands::CreateForm;
use csv::{Error as CsvError, ReaderBuilder};
use rusqlite::{Connection, Error as RError, Result, Row, Transaction, params, types::ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

// Представляет одну запись из таблицы data
#[derive(Debug, Serialize, Deserialize)]
pub struct DataRecord {
    pub id: u32,
    pub fields: HashMap<String, FieldValue>, // f_0, f_1 и т.д.
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
}

// Разные типы полей для гибкой обработки
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Null,
}

// Метаданные таблицы из таблицы meta
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct TableMeta {
    pub name: String,
    pub desc: String,
    pub field_names: HashMap<String, String>,    // f_0 -> "Name", f_1 -> "Age"
    pub field_types: HashMap<String, FieldType>, // f_0 -> Text, f_1 -> Number
    pub operations: Vec<Operation>,              // вычисляемые поля
    pub search_config: Vec<String>,              // настройки поиска
    pub count_data: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Operation {
    pub name: String, // "Total Price"
    pub description: String,
    pub expression: String, // "f_6 * 17 / f_20"
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
            // Присваивание Some автоматически дропает предыдущий Connection (закрывает файл)
            self.conn = Some(Connection::open(&path).map_err(|e| e.to_string())?);
            self.current_path = Some(path);
            // Пытаемся загрузить meta. Если ошибка - оставляем None.
            if let Some(conn_ref) = self.conn.as_ref() {
                self.meta = self.load_meta(conn_ref).ok();
            }
        }
        Ok(self.conn.as_mut().unwrap())
    }

    // теперь замыкание получает и соединение, и ссылку на meta
    pub fn with_conn<F, T>(&mut self, path: PathBuf, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection, Option<&TableMeta>) -> Result<T, String>,
    {
        self.get_conn(path)?;

        let conn = self.conn.as_ref().unwrap();
        let meta = self.meta.as_ref(); // Может быть None

        f(conn, meta)
    }

    fn load_meta(&self, conn: &Connection) -> rusqlite::Result<TableMeta> {
        let mut stmt = conn.prepare("SELECT key, value FROM meta")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;

        let mut meta_data: HashMap<String, String> = HashMap::new();
        for row in rows {
            let (key, value) = row?;
            meta_data.insert(key, value);
        }

        let name = meta_data.get("name").cloned().unwrap_or_default();
        let desc = meta_data.get("desc").cloned().unwrap_or_default();

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

        let search_config: Vec<String> = match meta_data.get("search_config") {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => Vec::new(),
        };

        let count_data: u32 = conn.query_row(&format!("SELECT COUNT(*) FROM \"{}\"", "data"), [], |r| r.get(0))?;

        Ok(TableMeta {
            name,
            desc,
            field_names,
            field_types,
            operations,
            search_config,
            count_data,
        })
    }

    pub fn update_meta<F>(&mut self, path: &PathBuf, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut TableMeta) -> Result<(), String>,
    {
        // Проверяем, что работаем с той же базой
        if self.current_path.as_ref() != Some(path) {
            return Err("Database not opened".to_string());
        }

        let meta = self.meta.as_mut().ok_or("No meta loaded")?;

        // Применяем изменения к meta в памяти
        f(meta)?;

        // Сохраняем изменения в БД (если нужно)
        Ok(())
    }
}

pub fn search_items(conn: &Connection, meta: &TableMeta, query: &str) -> Result<Vec<DataRecord>, String> {
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
    let conditions: Vec<String> = search_fields.iter().map(|f| format!("{} LIKE ?1", f)).collect();

    let where_clause = if conditions.is_empty() {
        String::from("")
    } else {
        format!("WHERE {}", conditions.join(" OR "))
    };

    let sql = format!("SELECT * FROM data {} ORDER BY id ASC LIMIT 10", where_clause);

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

pub fn save_search_config(conn: &Connection, vec: &Vec<String>) -> Result<()> {
    let config_json = serde_json::to_string(vec).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('search_config', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![config_json],
    )?;

    Ok(())
}

// Получить один элемент по ID
pub fn get_el(conn: &Connection, id: u32) -> Result<Option<DataRecord>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM data WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    match stmt.query_row([id], row_to_record) {
        Ok(record) => Ok(Some(record)),
        Err(RError::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn row_to_record(row: &Row) -> Result<DataRecord> {
    let id: u32 = row.get(0)?;
    let mut fields = HashMap::new();

    let column_count = row.as_ref().column_count();
    for i in 1..column_count {
        let name = row.as_ref().column_name(i)?.to_string();
        match row.get_ref_unwrap(i) {
            ValueRef::Null => {
                fields.insert(name, FieldValue::Null);
            }
            ValueRef::Integer(n) => {
                fields.insert(name, FieldValue::Number(n as f64));
            }
            ValueRef::Real(f) => {
                fields.insert(name, FieldValue::Number(f));
            }
            ValueRef::Text(b) => {
                let s = String::from_utf8_lossy(b).into_owned();
                fields.insert(name, FieldValue::Text(s));
            }
            ValueRef::Blob(_) => {
                fields.insert(name, FieldValue::Null);
            }
        }
    }
    Ok(DataRecord { id, fields })
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

    // Базовая meta для пустой базы
    let now = chrono::Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO meta (key, value) VALUES 
         ('name', ?1),
         ('desc', ?2),
         ('field_names', ?3),
         ('field_types', ?4),
         ('operations', ?5),
         ('search_config', ?6),
         ('created_at', ?7)",
        params![
            "",   // name - пустое, юзер сам заполнит
            "",   // desc - пустое
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

/*pub fn create_from_sheet(path: &PathBuf, val: &CreateForm) -> Result<(), String> {
    match val.file_extension.clone().as_str() {
        "csv" | "tsv" => create_from_csv_file(path, val),
        "xls" | "xlsx" | "ods" => create_from_sheet_file(path, val),
        &_ => {
            Err(RError::SqliteFailure(rusqlite::ffi::Error::new(1), Some("Unknown file ext".to_string())).to_string())
        }
    }
}*/

pub fn create_from_csv_file(val: &CreateForm) -> Result<(), String> {
    // Создаем пустую базу данных
    create_empty_database(&val.db_name)?;
    let mut conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;
    
    // Открываем CSV файл - читаем все строки как есть
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(val.file_path.as_ref().unwrap())
        .map_err(|e| e.to_string())?;
    
    // Читаем все строки из CSV
    let mut all_rows: Vec<csv::StringRecord> = Vec::new();
    for result in rdr.records() {
        all_rows.push(result.map_err(|e| e.to_string())?);
    }
    
    if all_rows.is_empty() {
        return Ok(());
    }
    
    // Разделяем заголовки и данные
    let (headers, data_rows) = if val.has_header {
        let headers = all_rows[0].iter().map(|s| s.to_string()).collect();
        let data_rows = &all_rows[1..];
        (headers, data_rows)
    } else {
        (Vec::new(), &all_rows[..])
    };
    
    if data_rows.is_empty() {
        return Ok(());
    }
    
    // Определяем количество колонок
    let col_count = data_rows[0].len();
    
    // Подготавливаем информацию о полях
    let mut field_names_map = HashMap::new();
    let mut field_types_map = HashMap::new();
    
    for i in 0..col_count {
        let first_value = data_rows[0].get(i).unwrap_or("");
        let field_type = if first_value.parse::<f64>().is_ok() { 
            FieldType::Number 
        } else { 
            FieldType::Text 
        };
        
        let display_name = if val.has_header && i < headers.len() {
            headers[i].clone()
        } else {
            format!("Field {}", i + 1)
        };
        
        let field_name = format!("f_{}", i);
        field_names_map.insert(field_name.clone(), display_name);
        field_types_map.insert(field_name, field_type);
    }
    
    // Добавляем колонки в таблицу data
    for i in 0..col_count {
        let field_name = format!("f_{}", i);
        let sql = format!("ALTER TABLE data ADD COLUMN {} TEXT", field_name);
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
    }
    
    // Импортируем данные в отдельном блоке
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        
        // Подготавливаем запросы
        let mut insert_stmt = tx.prepare("INSERT INTO data DEFAULT VALUES").map_err(|e| e.to_string())?;
        
        let mut update_stmts = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let field_name = format!("f_{}", i);
            let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_name);
            let mut stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
            update_stmts.push(stmt);
        }
        
        // Импортируем данные
        for row in data_rows {
            insert_stmt.execute([]).map_err(|e| e.to_string())?;
            let record_id = tx.last_insert_rowid();
            
            for (i, value) in row.iter().enumerate() {
                if let Some(stmt) = update_stmts.get_mut(i) {
                    stmt.execute(params![value, record_id]).map_err(|e| e.to_string())?;
                }
            }
        }
        
        // Явно уничтожаем Statement перед коммитом
        drop(insert_stmt);
        drop(update_stmts);
        
        tx.commit().map_err(|e| e.to_string())?;
    }
    
    // Обновляем meta таблицу
    let fields_display_json = serde_json::to_string(&field_names_map).map_err(|e| e.to_string())?;
    let fields_types_json = serde_json::to_string(&field_types_map).map_err(|e| e.to_string())?;
    let search_config_json = serde_json::to_string(&vec!["f_0"]).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'field_names'",
        params![fields_display_json],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'field_types'",
        params![fields_types_json],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![search_config_json],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'count_data'",
        params![data_rows.len().to_string()],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

pub fn create_from_sheet_file(path: &PathBuf, val: &CreateForm) -> Result<(), String> {
    // TODO: реализовать
    Ok(())
}

pub fn create_from_sqlite(path: &PathBuf, val: &CreateForm) -> Result<()> {
    // TODO: реализовать
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


// Сохраняем старую функцию для обратной совместимости со строковыми данными
/*pub fn import_csv(conn: &Connection, csv_data: &str, has_header: bool) -> Result<Vec<String>, String> {
    let rdr = match csv::ReaderBuilder::new()
        .has_headers(has_header)
        .flexible(true)
        .from_reader(csv_data.as_bytes()) {
            Ok(rdr) => rdr,
            Err(e) => return Err(format!("{}", e)),
        };

    import_csv_from_reader(conn, rdr, has_header)
}*/

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

// ==================== Create demo ====================

pub fn create_ballistics_database(path: &PathBuf) -> Result<(), String> {
    // Создаем пустую базу
    create_empty_database(path)?;

    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    // Обновляем name и desc
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'name'",
        params!["Ballistics Data"],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'desc'",
        params!["Ballistic trajectory calculator demo data"],
    )
    .map_err(|e| e.to_string())?;

    // add search field
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![serde_json::to_string(&vec!["f_0"]).unwrap()],
    )
    .map_err(|e| e.to_string())?;

    /*let rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .map_err(|e| e.to_string())?;
    create_from_csv_file(&conn, rdr, true)?;*/
    // Импортируем CSV с заголовком
    //let rdr = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_reader(BALLISTICS_CSV2.as_bytes());
    //import_csv_from_reader(&conn, rdr, true)?;
    //let rdr = csv::ReaderBuilder::new().has_headers(true).flexible(true).from_reader(BALLISTICS_CSV2.as_bytes());
    
    /*
        let rdr = csv::ReaderBuilder::new().has_headers(has_header).flexible(true).from_reader(csv_data.as_bytes());
    import_csv_from_reader(&conn, rdr, has_header)?;

    Вызов для файла:
    let rdr = csv::ReaderBuilder::new().has_headers(has_header).flexible(true).from_path(path)?;
    import_csv_from_reader(&conn, rdr, has_header)?;
         */

    import_csv(&conn, BALLISTICS_CSV, true).map_err(|e| e.to_string())?;

    // Добавляем операции
    add_operation(
        &conn,
        "Energy (J)",
        "f_1 * f_2 * f_2 / 2000", // bullet_mass_g * velocity^2 / 2000
        Some("Kinetic energy in Joules"),
    )
    .map_err(|e| e.to_string())?;

    add_operation(
        &conn,
        "Sectional Density",
        "f_1 / (f_4 * 1000)", // mass / cross_section_cm2 * 1000
        Some("Bullet mass / cross-sectional area"),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// Заглушки для будущих демо-баз
pub fn create_recipes_database(path: &PathBuf) -> Result<(), String> {
    // TODO: реализовать
    Ok(())
}

pub fn create_inventory_database(path: &PathBuf) -> Result<(), String> {
    // TODO: реализовать
    Ok(())
}

// Основная функция создания демо-базы по имени
pub fn create_example_database(name: &str, path: &PathBuf) -> Result<(), String> {
    match name {
        "example/ballistics.refer" => create_ballistics_database(path),
        "recipes" => create_recipes_database(path),
        "inventory" => create_inventory_database(path),
        _ => Err(RError::SqliteFailure(rusqlite::ffi::Error::new(1), Some("Unknown file ext".to_string())).to_string()),
    }
}

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

// Данные для баллистики
const BALLISTICS_CSV2: &str = ".308 Winchester,11.3,800,0.475,0.48
.338 Lapua Mag,16.2,900,0.648,0.57
7.62x54R LPS,9.6,830,0.420,0.48
5.45x39 7N6,3.4,900,0.347,0.23
.223 Remington,4.0,930,0.304,0.25
6.5 Creedmoor,8.9,860,0.520,0.34
.300 Win Mag,11.7,880,0.590,0.42
9x19 Parabellum,8.0,360,0.150,0.12
.45 ACP,15.0,260,0.180,0.16
12 gauge slug,28.0,480,0.210,2.15";
