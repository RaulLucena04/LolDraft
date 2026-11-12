use thiserror::Error;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("Error HTTP: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Respuesta inválida de la fuente: {0}")]
    InvalidData(String),

    #[error("No hay versiones de Data Dragon disponibles")]
    NoVersions,

    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
}
