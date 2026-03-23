use std::collections::HashMap;
use rusqlite::{Connection, params};
use calamine::{Reader, open_workbook_auto, Data};
use crate::{commands::CreateForm, sql};
use sql::{FieldType};


pub fn create_from_csv_file(val: &CreateForm) -> Result<(), String> {
    // Создаем пустую базу данных
    sql::create_empty_database(&val.db_name)?;
    let mut conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;
    
    // Автоопределение разделителя по расширению и содержимому
    let delimiter = if val.file_extension.to_lowercase() == "tsv" {
        b'\t'
    } else {
        // Для CSV пробуем определить разделитель по первой строке
        let content = std::fs::read_to_string(val.file_path.as_ref().unwrap())
            .map_err(|e| e.to_string())?;
        let first_line = content.lines().next().unwrap_or("");
        
        // Подсчитываем частоту возможных разделителей
        let comma_count = first_line.matches(',').count();
        let semicolon_count = first_line.matches(';').count();
        let pipe_count = first_line.matches('|').count();
        let tab_count = first_line.matches('\t').count();
        
        // Выбираем самый частый разделитель
        let max_count = *[comma_count, semicolon_count, pipe_count, tab_count].iter().max().unwrap_or(&0);
        
        if max_count == 0 {
            b',' // если нет ни одного разделителя, используем запятую по умолчанию
        } else if semicolon_count == max_count {
            b';'
        } else if pipe_count == max_count {
            b'|'
        } else if tab_count == max_count {
            b'\t'
        } else {
            b',' // по умолчанию запятая
        }
    };
    
    // Открываем CSV файл с определенным разделителем
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
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
            let stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
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

pub fn create_from_sheet_file(val: &CreateForm) -> Result<(), String> {
    // Открываем файл
    let mut workbook = open_workbook_auto(val.file_path.as_ref().unwrap())
        .map_err(|e| e.to_string())?;
    
    // Получаем список листов
    let sheet_names = workbook.sheet_names().to_vec();
    
    if sheet_names.is_empty() {
        return Err("No sheets found in file".to_string());
    }
    
    // Всегда берем первый лист
    let sheet_name = &sheet_names[0];
    
    // Если листов несколько, просто логируем (можно убрать в релизе)
    if sheet_names.len() > 1 {
        tracing::warn!("Info: Using first sheet '{}', ignoring others: {:?}", sheet_name, &sheet_names[1..]);
    }
    
    // Читаем данные из первого листа
    let range = workbook.worksheet_range(sheet_name)
        .map_err(|e| e.to_string())?;
    
    // Конвертируем в строки
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    
    for row in range.rows() {
        let mut string_row = Vec::with_capacity(row.len());
        for cell in row {
            let value = match cell {
                Data::String(s) => s.clone(),
                Data::Float(f) => f.to_string(),
                Data::Int(i) => i.to_string(),
                Data::Bool(b) => b.to_string(),
                Data::DateTime(dt) => dt.to_string(),
                Data::Empty => String::new(),
                _ => String::new(),
            };
            string_row.push(value);
        }
        all_rows.push(string_row);
    }
    
    if all_rows.is_empty() {
        return Ok(());
    }
    
    // Разделяем заголовки и данные
    let (headers, data_rows) = if val.has_header {
        let headers = all_rows[0].clone();
        let data_rows = &all_rows[1..];
        (headers, data_rows)
    } else {
        (Vec::new(), &all_rows[..])
    };
    
    if data_rows.is_empty() {
        return Ok(());
    }
    
    // Создаем пустую базу данных
    sql::create_empty_database(&val.db_name)?;
    let mut conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;
    
    // Определяем количество колонок
    let col_count = data_rows[0].len();
    
    // Подготавливаем информацию о полях
    let mut field_names_map = HashMap::new();
    let mut field_types_map = HashMap::new();
    
    for i in 0..col_count {
        let first_value = &data_rows[0][i];
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
    
    // Импортируем данные в транзакции
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        
        let mut insert_stmt = tx.prepare("INSERT INTO data DEFAULT VALUES").map_err(|e| e.to_string())?;
        
        let mut update_stmts = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let field_name = format!("f_{}", i);
            let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_name);
            let stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
            update_stmts.push(stmt);
        }
        
        for row in data_rows {
            insert_stmt.execute([]).map_err(|e| e.to_string())?;
            let record_id = tx.last_insert_rowid();
            
            for (i, value) in row.iter().enumerate() {
                if let Some(stmt) = update_stmts.get_mut(i) {
                    stmt.execute(params![value, record_id]).map_err(|e| e.to_string())?;
                }
            }
        }
        
        drop(insert_stmt);
        drop(update_stmts);
        
        tx.commit().map_err(|e| e.to_string())?;
    }
    
    // Обновляем meta
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

pub fn create_from_sqlite(val: &CreateForm) -> Result<(), String> {
    // Открываем исходную базу данных
    let source_conn = Connection::open(val.file_path.as_ref().unwrap())
        .map_err(|e| e.to_string())?;
    
    // Получаем список всех таблиц
    let mut stmt = source_conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    ).map_err(|e| e.to_string())?;
    
    let tables: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    
    if tables.is_empty() {
        return Err("No user tables found in SQLite file".to_string());
    }
    
    // Берем первую таблицу
    let table_name = &tables[0];
    
    if tables.len() > 1 {
        tracing::warn!("Info: Using first table '{}', ignoring others: {:?}", table_name, &tables[1..]);
    }
    
    // Получаем информацию о колонках
    let mut stmt = source_conn.prepare(&format!("PRAGMA table_info({})", table_name))
        .map_err(|e| e.to_string())?;
    
    let columns: Vec<(String, String)> = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    }).map_err(|e| e.to_string())?
      .collect::<Result<_, _>>()
      .map_err(|e| e.to_string())?;
    
    // Создаем новую базу данных
    sql::create_empty_database(&val.db_name)?;
    let mut dest_conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;
    
    // Подготавливаем информацию о полях
    let mut field_names_map = HashMap::new();
    let mut field_types_map = HashMap::new();
    
    for (i, (col_name, col_type)) in columns.iter().enumerate() {
        let field_name = format!("f_{}", i);
        
        // Определяем тип поля
        let field_type = if col_type.to_lowercase().contains("int") || 
                           col_type.to_lowercase().contains("real") || 
                           col_type.to_lowercase().contains("float") || 
                           col_type.to_lowercase().contains("double") || 
                           col_type.to_lowercase().contains("numeric") {
            FieldType::Number
        } else {
            FieldType::Text
        };
        
        field_names_map.insert(field_name.clone(), col_name.clone());
        field_types_map.insert(field_name.clone(), field_type);
        
        // Добавляем колонку в таблицу data
        let sql = format!("ALTER TABLE data ADD COLUMN {} TEXT", field_name);
        dest_conn.execute(&sql, []).map_err(|e| e.to_string())?;
    }
    
    // Читаем данные из исходной таблицы
    let mut stmt = source_conn.prepare(&format!("SELECT * FROM {}", table_name))
        .map_err(|e| e.to_string())?;
    
    let column_count = columns.len();
    
    // Импортируем данные в транзакции
    {
        let tx = dest_conn.transaction().map_err(|e| e.to_string())?;
        
        let mut insert_stmt = tx.prepare("INSERT INTO data DEFAULT VALUES").map_err(|e| e.to_string())?;
        
        let mut update_stmts = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let field_name = format!("f_{}", i);
            let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_name);
            let stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
            update_stmts.push(stmt);
        }
        
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut row_count = 0;
        
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            insert_stmt.execute([]).map_err(|e| e.to_string())?;
            let record_id = tx.last_insert_rowid();
            
            for i in 0..column_count {
                let value: String = match row.get(i) {
                    Ok(v) => v,
                    Err(_) => String::new(),
                };
                
                if let Some(stmt) = update_stmts.get_mut(i) {
                    stmt.execute(params![value, record_id]).map_err(|e| e.to_string())?;
                }
            }
            row_count += 1;
        }
        
        drop(insert_stmt);
        drop(update_stmts);
        
        tx.commit().map_err(|e| e.to_string())?;
        
        // Обновляем count_data
        dest_conn.execute(
            "UPDATE meta SET value = ?1 WHERE key = 'count_data'",
            params![row_count.to_string()],
        ).map_err(|e| e.to_string())?;
    }
    
    // Обновляем meta
    let fields_display_json = serde_json::to_string(&field_names_map).map_err(|e| e.to_string())?;
    let fields_types_json = serde_json::to_string(&field_types_map).map_err(|e| e.to_string())?;
    let search_config_json = serde_json::to_string(&vec!["f_0"]).map_err(|e| e.to_string())?;
    
    dest_conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'field_names'",
        params![fields_display_json],
    ).map_err(|e| e.to_string())?;
    
    dest_conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'field_types'",
        params![fields_types_json],
    ).map_err(|e| e.to_string())?;
    
    dest_conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![search_config_json],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

/*pub fn run_import(val:&CreateForm/*source: &Path, target_db: &str, has_header: bool*/) -> Result<(), String> {
    let _ = sql::create_empty_database(&val.db_name);
    let scan = prepare_meta(val)?;
    if 1 < 2 { return Err("scan exit".into()); }
    let mut conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let col_count = scan.headers.len();

    // 1. Подготовка Meta
    let mut field_names = HashMap::new();
    let mut field_types = HashMap::new();
    for (i, name) in scan.headers.iter().enumerate() {
        let f_key = format!("f_{}", i);
        field_names.insert(f_key.clone(), name.clone());
        field_types.insert(f_key, scan.types[i].clone());
    }

    let names_json = serde_json::to_string(&field_names).unwrap();
    let types_json = serde_json::to_string(&field_types).unwrap();

    tx.execute("UPDATE meta SET value = ?1 WHERE key = 'field_names'", [names_json])
        .map_err(|e| e.to_string())?;

    /*tx.execute("UPDATE meta SET field_names = ?1, field_types = ?2", [names_json, types_json])
        .map_err(|e| e.to_string())?;*/

    // 2. Наращивание таблицы data
    for (i, t) in scan.types.iter().enumerate() {
        let sql_type = match t { FieldType::Number => "REAL", FieldType::Text => "TEXT" };
        let _ = tx.execute(&format!("ALTER TABLE data ADD COLUMN f_{} {}", i, sql_type), []);
    }

    // 3. Быстрая вставка
    let placeholders = (1..=col_count).map(|i| format!("?{}", i)).collect::<Vec<_>>().join(",");
    let sql = format!("INSERT INTO data ({}) VALUES ({})", 
        (0..col_count).map(|i| format!("f_{}", i)).collect::<Vec<_>>().join(","),
        placeholders
    );

    {
        let mut stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
        
        // Вставляем сначала те 100 строк, что уже в памяти, затем остальное (если CSV)
        for row in scan.first_rows {
            let params: Vec<&dyn ToSql> = row.iter().map(|s| s as &dyn ToSql).collect();
            stmt.execute(&*params).map_err(|e| e.to_string())?;
        }
        
        // Тут можно добавить итератор для остатка файла, если он большой...
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}*/

// Скан файла для определения колонок и типов
/* pub fn prepare_meta(val: &CreateForm) -> Result<ImportScan, String> {
    let mut raw_rows: Vec<Vec<String>> = match val.file_extension.as_ref() {
        "csv" | "tsv" => {
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true) // Чтобы не падать на кривых строках
                .from_path(val.file_path.as_ref().unwrap()).map_err(|e| e.to_string())?;
            
            rdr.records().take(2) // Берем 2 строки для анализа типов
                .map(|r| r.map(|rec| rec.iter().map(|s| s.to_string()).collect()))
                .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
        }
        "xlsx" | "ods" | "xls" => {
            let mut excel = open_workbook_auto(val.file_path.as_ref().unwrap()).map_err(|e| e.to_string())?;
            let sheet = excel.sheet_names().first().cloned().ok_or("No sheets")?;
            let range = excel.worksheet_range(&sheet).map_err(|e| e.to_string())?;
            range.rows().take(2).map(|r| r.iter().map(format_data).collect()).collect()
        }
        _ => return Err("Unsupported format".into()),
    };

    if raw_rows.is_empty() || raw_rows.len() < 2 { return Err("File is empty".into()); }

    let headers = if val.has_header {
        raw_rows[0].clone()
    } else {
        (0..raw_rows[0].len()).map(|i| format!("Field {}", i)).collect()
    };

    // Определяем типы по второй строке raw_rows
    let types = raw_rows[1].iter().map(|val| {
        if val.parse::<f64>().is_ok() { FieldType::Number } else { FieldType::Text }
    }).collect();
    println!("raw_rows: {:?}",&raw_rows);
    println!("headers: {:?}, types {:?}",&headers,&types);

    Ok(ImportScan { headers, types, first_rows: raw_rows })
}*/
