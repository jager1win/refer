use std::fs;
use tauri::Manager;
use crate::errors::*;
use crate::{SettingsStore};

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::AppHandle;

pub struct DbState {
  pub conn: Mutex<Connection>,
}

impl DbState {
  pub fn new(path: &std::path::Path) -> Result<Self, RError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", &"WAL")?;
    conn.pragma_update(None, "foreign_keys", &1)?;
    Ok(Self { conn: Mutex::new(conn) })
  }
}