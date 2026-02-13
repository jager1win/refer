use crate::StatisticsState;
use std::{fs,io};
use tracing::{error,info};
use rusqlite::{Connection, Result, Error};
use std::path::{Path, PathBuf};

// Единая функция создания любой новой базы
pub fn create_empty_database(full_path: &Path) -> Result<(), Error> {
    // Создаем родительскую директорию, если её нет
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1), 
                Some(format!("Cannot create directory: {}", e))
            ))?;
    }
    
    // Проверяем, что файл не существует (не перезаписываем случайно)
    if full_path.exists() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(19), // SQLITE_CONSTRAINT
            Some(format!("Database already exists: {}", full_path.display()))
        ));
    }
    
    let conn = Connection::open(full_path)?;
    
    // 1. Таблица items - только служебные поля + данные
    conn.execute(
        "CREATE TABLE items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,  -- единственное поле по умолчанию
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // 2. Триггер для автообновления updated_at
    conn.execute(
        "CREATE TRIGGER update_items_timestamp 
         AFTER UPDATE ON items
         BEGIN
            UPDATE items SET updated_at = CURRENT_TIMESTAMP 
            WHERE id = NEW.id;
         END;",
        [],
    )?;

    // 3. Таблица ctrl - метаданные о колонках
    conn.execute(
        "CREATE TABLE ctrl (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            field_name TEXT NOT NULL UNIQUE,  -- техническое имя (f0, f1... или для name - 'name')
            display_name TEXT NOT NULL,       -- 'Название', 'Высота' и т.д.
            field_type TEXT DEFAULT 'text',   -- text, integer, real, computed
            is_system BOOLEAN DEFAULT 0,      -- системное поле (id, timestamps)
            is_editable BOOLEAN DEFAULT 1,
            display_order INTEGER,
            visible BOOLEAN DEFAULT 1,
            description TEXT,
            -- для вычисляемых полей
            expression TEXT,
            result_type TEXT DEFAULT 'text',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // 4. Добавляем метаданные для системных полей
    conn.execute(
        "INSERT INTO ctrl (field_name, display_name, field_type, is_system, is_editable, display_order) VALUES
         ('id', 'ID', 'integer', 1, 0, 0),
         ('name', 'Название', 'text', 0, 1, 1),
         ('created_at', 'Дата создания', 'datetime', 1, 0, 2),
         ('updated_at', 'Дата изменения', 'datetime', 1, 0, 3)",
        [],
    )?;

    // 5. Таблица для описания справочника
    conn.execute(
        "CREATE TABLE db_info (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )?;
    
    // Сохраняем оригинальное имя файла без пути в db_info
    let file_name = full_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Новый справочник");
    
    conn.execute(
        "INSERT INTO db_info (key, value) VALUES 
         ('name', ?1),
         ('file_path', ?2),
         ('description', ''),
         ('version', '1.0')",
        rusqlite::params![file_name, full_path.to_string_lossy()],
    )?;
    
    Ok(())
}

pub fn create_example_database(path:&PathBuf) -> Result<(), Error> {

    create_empty_database(&path);

    Ok(())
}