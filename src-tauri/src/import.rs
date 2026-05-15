use std::collections::HashMap;

use crate::sql::FieldDef;
use crate::{commands::CreateForm, sql};
use calamine::{Data, Reader, open_workbook_auto};
use rusqlite::{Connection, params};

pub fn create_from_csv_file(val: &CreateForm) -> Result<(), String> {
    // Создаем пустую базу данных
    sql::create_empty_database(&val.db_name)?;
    let mut conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;

    // Автоопределение разделителя по расширению и содержимому
    let delimiter = if val.file_extension.to_lowercase() == "tsv" {
        b'\t'
    } else {
        // Для CSV пробуем определить разделитель по первой строке
        let content = std::fs::read_to_string(val.file_path.as_ref().unwrap()).map_err(|e| e.to_string())?;
        let first_line = content.lines().next().unwrap_or("");

        // Подсчитываем частоту возможных разделителей
        let comma_count = first_line.matches(',').count();
        let semicolon_count = first_line.matches(';').count();
        let pipe_count = first_line.matches('|').count();
        let tab_count = first_line.matches('\t').count();

        // Выбираем самый частый разделитель
        let max_count = *[comma_count, semicolon_count, pipe_count, tab_count]
            .iter()
            .max()
            .unwrap_or(&0);

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
    let mut fields_map = HashMap::new();

    for i in 0..col_count {
        let first_value = data_rows[0].get(i).unwrap_or("");
        let field_type = if first_value.parse::<f64>().is_ok() {
            "number"
        } else {
            "text"
        };

        let display_name = if val.has_header && i < headers.len() {
            headers[i].clone()
        } else {
            format!("Field {}", i + 1)
        };

        let field_name = format!("f_{}", i);
        fields_map.insert(
            field_name.clone(),
            FieldDef {
                name: display_name,
                ftype: field_type.to_string(),
            },
        );
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
        let mut insert_stmt = tx
            .prepare("INSERT INTO data DEFAULT VALUES")
            .map_err(|e| e.to_string())?;

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
    let fields_json = serde_json::to_string(&fields_map).map_err(|e| e.to_string())?;
    let search_config_json = serde_json::to_string(&vec!["f_0"]).map_err(|e| e.to_string())?;

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'fields'", params![fields_json])
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![search_config_json],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_from_sheet_file(val: &CreateForm) -> Result<(), String> {
    // Открываем файл
    let mut workbook = open_workbook_auto(val.file_path.as_ref().unwrap()).map_err(|e| e.to_string())?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("No sheets found in file".to_string());
    }

    let sheet_name = &sheet_names[0];
    if sheet_names.len() > 1 {
        tracing::warn!(
            "Warn: Using first sheet '{}', ignoring others: {:?}",
            sheet_name,
            &sheet_names[1..]
        );
    }

    // Читаем данные из первого листа
    let range = workbook.worksheet_range(sheet_name).map_err(|e| e.to_string())?;
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
    let mut fields_map = HashMap::new();

    for i in 0..col_count {
        let first_value = &data_rows[0][i];
        let field_type = if first_value.parse::<f64>().is_ok() {
            "number"
        } else {
            "text"
        };

        let display_name = if val.has_header && i < headers.len() {
            headers[i].clone()
        } else {
            format!("Field {}", i + 1)
        };

        let field_name = format!("f_{}", i);
        fields_map.insert(
            field_name.clone(),
            FieldDef {
                name: display_name,
                ftype: field_type.to_string(),
            },
        );
    }

    // Добавляем колонки в таблицу data
    for i in 0..col_count {
        let field_name = format!("f_{}", i);
        let sql = format!("ALTER TABLE data ADD COLUMN {} TEXT", field_name);
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
    }

    // ОПТИМИЗАЦИЯ: Используем INSERT вместо INSERT + UPDATE
    let field_list: Vec<String> = (0..col_count).map(|i| format!("f_{}", i)).collect();
    let placeholders: Vec<String> = (0..col_count).map(|_| "?".to_string()).collect();

    let insert_sql = format!(
        "INSERT INTO data ({}) VALUES ({})",
        field_list.join(", "),
        placeholders.join(", ")
    );

    // Импортируем данные в транзакции
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        {
            let mut insert_stmt = tx.prepare(&insert_sql).map_err(|e| e.to_string())?;

            // Вставляем данные пачками
            let batch_size = 1000;
            let mut batch_count = 0;

            for row in data_rows {
                // Преобразуем Vec<String> в Vec<&dyn ToSql>
                let params: Vec<&dyn rusqlite::ToSql> = row.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

                insert_stmt.execute(&*params).map_err(|e| e.to_string())?;
                batch_count += 1;

                // Каждые batch_size строк делаем checkpoint
                if batch_count % batch_size == 0 {
                    // Продолжаем в той же транзакции
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    // Обновляем meta
    let fields_json = serde_json::to_string(&fields_map).map_err(|e| e.to_string())?;
    let search_config_json = serde_json::to_string(&vec!["f_0"]).map_err(|e| e.to_string())?;

    conn.execute("UPDATE meta SET value = ?1 WHERE key = 'fields'", params![fields_json])
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
        params![search_config_json],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'count_data'",
        params![data_rows.len().to_string()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_from_sqlite(val: &CreateForm) -> Result<(), String> {
    // Открываем исходную базу данных
    let source_conn = Connection::open(val.file_path.as_ref().unwrap()).map_err(|e| e.to_string())?;

    // Получаем список всех таблиц
    let mut stmt = source_conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|e| e.to_string())?;

    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    if tables.is_empty() {
        return Err("No user tables found in SQLite file".to_string());
    }

    // Берем первую таблицу
    let table_name = &tables[0];

    if tables.len() > 1 {
        tracing::warn!(
            "Info: Using first table '{}', ignoring others: {:?}",
            table_name,
            &tables[1..]
        );
    }

    // Получаем информацию о колонках
    let mut stmt = source_conn
        .prepare(&format!("PRAGMA table_info({})", table_name))
        .map_err(|e| e.to_string())?;

    let columns: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    // Создаем новую базу данных
    sql::create_empty_database(&val.db_name)?;
    let mut dest_conn = Connection::open(&val.db_name).map_err(|e| e.to_string())?;

    // Подготавливаем информацию о полях
    let mut fields_map = HashMap::new();

    for (i, (col_name, col_type)) in columns.iter().enumerate() {
        let field_name = format!("f_{}", i);

        // Определяем тип поля
        let field_type = if col_type.to_lowercase().contains("int")
            || col_type.to_lowercase().contains("real")
            || col_type.to_lowercase().contains("float")
            || col_type.to_lowercase().contains("double")
            || col_type.to_lowercase().contains("numeric")
        {
            "number"
        } else {
            "text"
        };

        fields_map.insert(
            field_name.clone(),
            FieldDef {
                name: col_name.clone(),
                ftype: field_type.to_string(),
            },
        );

        // Добавляем колонку в таблицу data
        let sql = format!("ALTER TABLE data ADD COLUMN {} TEXT", field_name);
        dest_conn.execute(&sql, []).map_err(|e| e.to_string())?;
    }

    // Читаем данные из исходной таблицы
    let mut stmt = source_conn
        .prepare(&format!("SELECT * FROM {}", table_name))
        .map_err(|e| e.to_string())?;

    let column_count = columns.len();

    // Импортируем данные в транзакции
    {
        let tx = dest_conn.transaction().map_err(|e| e.to_string())?;

        let mut insert_stmt = tx
            .prepare("INSERT INTO data DEFAULT VALUES")
            .map_err(|e| e.to_string())?;

        let mut update_stmts = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let field_name = format!("f_{}", i);
            let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_name);
            let stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
            update_stmts.push(stmt);
        }

        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            insert_stmt.execute([]).map_err(|e| e.to_string())?;
            let record_id = tx.last_insert_rowid();

            for i in 0..column_count {
                let value: String = row.get(i).unwrap_or_default();

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
    let fields_json = serde_json::to_string(&fields_map).map_err(|e| e.to_string())?;
    let search_config_json = serde_json::to_string(&vec!["f_0"]).map_err(|e| e.to_string())?;

    dest_conn
        .execute("UPDATE meta SET value = ?1 WHERE key = 'fields'", params![fields_json])
        .map_err(|e| e.to_string())?;

    dest_conn
        .execute(
            "UPDATE meta SET value = ?1 WHERE key = 'search_config'",
            params![search_config_json],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}
