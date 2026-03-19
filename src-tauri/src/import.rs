use std::io::Cursor;
use rusqlite::{Connection, params, Error as RusqliteError};

// Типы ошибок для импорта
#[derive(Debug)]
pub enum ImportError {
    Rusqlite(RusqliteError),
    Csv(String),
    InvalidData(String),
}

impl From<RusqliteError> for ImportError {
    fn from(err: RusqliteError) -> Self {
        ImportError::Rusqlite(err)
    }
}

// Основная функция импорта, принимающая CreateForm
pub fn import_from_form(conn: &Connection, form: &crate::commands::CreateForm) -> Result<Vec<String>, ImportError> {
    match form.mode.as_str() {
        "empty" => Ok(Vec::new()), // просто создаем пустую БД
        
        "sheet" => {
            let file_data = form.file_data.as_ref()
                .ok_or_else(|| ImportError::InvalidData("No file data provided".to_string()))?;
            
            let extension = form.file_extension.as_deref()
                .unwrap_or("csv")
                .to_lowercase();
            
            match extension.as_str() {
                "csv" => import_csv(conn, file_data, form.has_header),
                "tsv" => import_tsv(conn, file_data, form.has_header),
                "xls" | "xlsx" => {
                    // Здесь будет импорт из Excel
                    Err(ImportError::InvalidData("Excel import not implemented yet".to_string()))
                }
                "ods" => {
                    // Импорт из OpenDocument
                    Err(ImportError::InvalidData("ODS import not implemented yet".to_string()))
                }
                _ => Err(ImportError::InvalidData(format!(
                    "Unsupported file extension: {}", extension
                ))),
            }
        }
        
        "sqlite" => {
            // Импорт из другой SQLite базы
            let file_data = form.file_data.as_ref()
                .ok_or_else(|| ImportError::InvalidData("No file data provided".to_string()))?;
            import_from_sqlite(conn, file_data)
        }
        
        _ => Err(ImportError::InvalidData(format!(
            "Unknown mode: {}", form.mode
        ))),
    }
}

// Импорт CSV
pub fn import_csv(conn: &Connection, data: &[u8], has_header: bool) -> Result<Vec<String>, ImportError> {
    let content = std::str::from_utf8(data)
        .map_err(|e| ImportError::InvalidData(format!("Invalid UTF-8 in CSV: {}", e)))?;
    
    // Используем существующую функцию, но адаптируем её под работу со строками
    import_csv_str(conn, content, has_header)
}

// Импорт TSV
pub fn import_tsv(conn: &Connection, data: &[u8], has_header: bool) -> Result<Vec<String>, ImportError> {
    let content = std::str::from_utf8(data)
        .map_err(|e| ImportError::InvalidData(format!("Invalid UTF-8 in TSV: {}", e)))?;
    
    import_tsv_str(conn, content, has_header)
}

// Ваша оригинальная функция для CSV из &str (с небольшими улучшениями)
pub fn import_csv_str(conn: &Connection, csv_data: &str, has_header: bool) -> Result<Vec<String>, ImportError> {
    import_delimited_str(conn, csv_data, has_header, ',')
}

pub fn import_tsv_str(conn: &Connection, tsv_data: &str, has_header: bool) -> Result<Vec<String>, ImportError> {
    import_delimited_str(conn, tsv_data, has_header, '\t')
}

// Обобщённая функция для любого разделителя
fn import_delimited_str(
    conn: &Connection, 
    data: &str, 
    has_header: bool,
    delimiter: char
) -> Result<Vec<String>, ImportError> {
    let mut lines = data.lines();
    let mut field_names = Vec::new();

    // Обработка заголовка
    let headers: Vec<String> = if has_header {
        if let Some(header_line) = lines.next() {
            parse_line(header_line, delimiter)
        } else {
            Vec::new()
        }
    } else {
        // Если нет заголовка, создаем заглушки по первой строке данных
        if let Some(first_line) = lines.clone().next() {
            let col_count = parse_line(first_line, delimiter).len();
            (0..col_count)
                .map(|i| format!("#{}", i + 1))
                .collect()
        } else {
            Vec::new()
        }
    };

    // Получаем первую строку данных для определения типов
    let first_data_line = if has_header {
        lines.clone().next()
    } else {
        lines.clone().next()
    };

    // Определяем типы полей
    let field_types = if let Some(first_line) = first_data_line {
        let values = parse_line(first_line, delimiter);
        determine_field_types(&values, headers.len())
    } else {
        vec!["text".to_string(); headers.len()]
    };

    // Создаем поля в базе
    for (i, (header, field_type)) in headers.iter().zip(field_types.iter()).enumerate() {
        let field_name = add_field(conn, i, Some(header), field_type)?;
        field_names.push(field_name);
    }

    // Импортируем данные
    for line in lines {
        if !line.trim().is_empty() {
            import_data_line(conn, &field_names, line, delimiter)?;
        }
    }

    Ok(field_names)
}

// Парсинг строки с учётом кавычек
fn parse_line(line: &str, delimiter: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    // Экранированная кавычка
                    current.push('"');
                    chars.next(); // пропускаем вторую кавычку
                } else {
                    in_quotes = !in_quotes;
                }
            }
            _ if c == delimiter && !in_quotes => {
                result.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    result.push(current.trim().to_string());
    
    result
}

// Определение типов полей по первой строке данных
fn determine_field_types(values: &[String], expected_count: usize) -> Vec<String> {
    let mut types = Vec::with_capacity(expected_count);
    
    for i in 0..expected_count {
        if i < values.len() {
            types.push(detect_value_type(&values[i]));
        } else {
            types.push("text".to_string());
        }
    }
    
    types
}

// Определение типа отдельного значения
fn detect_value_type(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "text".to_string()
    } else if trimmed.parse::<i64>().is_ok() {
        "integer".to_string()
    } else if trimmed.parse::<f64>().is_ok() {
        "number".to_string()
    } else {
        "text".to_string()
    }
}

// Импорт одной строки данных
fn import_data_line(
    conn: &Connection,
    field_names: &[String],
    line: &str,
    delimiter: char
) -> Result<(), ImportError> {
    let values = parse_line(line, delimiter);
    
    conn.execute("INSERT INTO data DEFAULT VALUES", [])?;
    let record_id = conn.last_insert_rowid();
    
    for (i, value) in values.iter().enumerate() {
        if i < field_names.len() && !value.is_empty() {
            let sql = format!("UPDATE data SET {} = ?1 WHERE id = ?2", field_names[i]);
            conn.execute(&sql, params![value, record_id])?;
        }
    }
    
    Ok(())
}

// Заглушка для импорта из SQLite
fn import_from_sqlite(conn: &Connection, data: &[u8]) -> Result<Vec<String>, ImportError> {
    // TODO: реализовать импорт из другой SQLite базы
    Err(ImportError::InvalidData("SQLite import not implemented yet".to_string()))
}

// Заглушка для add_field (ваша существующая функция)
fn add_field(conn: &Connection, index: usize, name: Option<&String>, field_type: &str) -> Result<String, RusqliteError> {
    // Здесь должна быть ваша реальная реализация
    Ok(format!("field_{}", index))
}