use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, ToSql};
use calamine::{Reader, open_workbook_auto, Data};
use crate::{commands::CreateForm, sql};
use sql::{FieldType, FieldValue};

pub struct ImportScan {
    pub headers: Vec<String>,
    pub types: Vec<FieldType>,
    pub first_rows: Vec<Vec<String>>, // Буфер для предпросмотра и вставки
}

pub fn run_import(val:&CreateForm/*source: &Path, target_db: &str, has_header: bool*/) -> Result<(), String> {
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
}

// Скан файла для определения колонок и типов
pub fn prepare_meta(val: &CreateForm) -> Result<ImportScan, String> {
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
}


fn format_data(d: &Data) -> String {
    match d {
        Data::String(s) => s.to_string(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        _ => "".to_string(),
    }
}
/*

raw_rows: [["Код", "Наименование групп занятий"], ["0.0", "ВОЕННОСЛУЖАЩИЕ"]]
headers: ["Field 0", "Field 1"], types [Number, Text]
*/