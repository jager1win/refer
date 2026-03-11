use serde::Serialize;
use tracing::error;

#[derive(Serialize, Debug)]
pub struct RError(pub &'static str);

impl RError {
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }
}

impl From<std::io::Error> for RError {
    fn from(e: std::io::Error) -> Self {
        error!(error = %e, "io error");
        RError::new("err_io")
    }
}

impl From<rusqlite::Error> for RError {
    fn from(e: rusqlite::Error) -> Self {
        error!(error = %e, "sql error");
        RError::new("err_sqlite")
    }
}

impl From<csv::Error> for RError {
    fn from(e: csv::Error) -> Self {
        error!(error = %e, "csv error");
        RError::new("err_csv")
    }
}
