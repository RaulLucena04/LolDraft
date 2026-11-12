use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Error de JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No se encontró el registro: {0}")]
    NotFound(String),

    #[error("Dato inválido: {0}")]
    Invalid(String),
}
