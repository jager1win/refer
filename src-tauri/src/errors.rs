use serde::Serialize;

#[derive(Serialize)]
pub struct RError(pub String);

impl RError {
    pub fn new(code: &str) -> Self {Self(code.to_string()) }
}

impl From<std::io::Error> for RError {
  fn from(_: std::io::Error) -> Self { RError::new("IO_ERROR") }
}

impl From<rusqlite::Error> for RError {
  fn from(_: rusqlite::Error) -> Self { RError::new("SQL_ERROR") }
}

impl From<csv::Error> for RError {
  fn from(_: csv::Error) -> Self { RError::new("CSV_ERROR") }
}