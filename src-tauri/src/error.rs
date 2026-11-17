use loldraft_data::DataError;
use loldraft_domain::DraftError;
use loldraft_engine::{AnalysisError, ProfileError, ProposalError, RecommendationError};
use loldraft_sources::SourceError;
use serde::Serialize;
use std::fmt::Display;

/// Error estable para la UI. El `code` permite reaccionar sin comparar textos;
/// el `message` está en español y describe qué pasó sin filtrar detalles internos.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn internal(context: impl Display) -> Self {
        Self::new(
            "internal",
            format!("Error interno al procesar la solicitud: {context}"),
        )
    }

    pub fn no_active_draft() -> Self {
        Self::new(
            "no_active_draft",
            "No hay un draft abierto. Crea uno nuevo o carga una sesión.",
        )
    }
}

impl From<DraftError> for CommandError {
    fn from(error: DraftError) -> Self {
        match error {
            DraftError::Complete => Self::new("draft_complete", error.to_string()),
            DraftError::WrongStep {
                expected_side,
                expected_kind,
                actual_side,
                actual_kind,
            } => Self::new(
                "wrong_step",
                format!(
                    "Fuera de turno: se esperaba {} {} y se recibió {} {}.",
                    expected_side.label_es(),
                    expected_kind.as_str(),
                    actual_side.label_es(),
                    actual_kind.as_str()
                ),
            ),
            DraftError::ChampionAlreadyUsed(key) => Self::new(
                "champion_already_used",
                format!("El campeón {key} ya está usado en este draft."),
            ),
            DraftError::UnknownChampion(key) => Self::new(
                "unknown_champion",
                format!("El campeón {key} no existe en el catálogo local."),
            ),
            DraftError::NoNextStep => Self::new(
                "draft_complete",
                "No queda ninguna acción disponible en este draft.",
            ),
        }
    }
}

impl From<DataError> for CommandError {
    fn from(error: DataError) -> Self {
        Self::new(
            "data_error",
            format!("No se pudo acceder a los datos locales: {error}"),
        )
    }
}

impl From<SourceError> for CommandError {
    fn from(error: SourceError) -> Self {
        Self::new(
            "source_error",
            format!("No se pudo sincronizar con Data Dragon: {error}"),
        )
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(
            "serialization_error",
            format!("No se pudo interpretar los datos guardados: {error}"),
        )
    }
}

impl From<ProposalError> for CommandError {
    fn from(error: ProposalError) -> Self {
        Self::new(
            "profile_proposal_invalid",
            format!("La propuesta de perfiles no es válida: {error}"),
        )
    }
}

impl From<ProfileError> for CommandError {
    fn from(error: ProfileError) -> Self {
        Self::new(
            "profile_set_invalid",
            format!("El conjunto de perfiles no es válido: {error}"),
        )
    }
}

impl From<AnalysisError> for CommandError {
    fn from(error: AnalysisError) -> Self {
        Self::new(
            "analysis_error",
            format!("No se pudo analizar la composición: {error}"),
        )
    }
}

impl From<RecommendationError> for CommandError {
    fn from(error: RecommendationError) -> Self {
        Self::new(
            "recommendation_error",
            format!("No se pudieron calcular las recomendaciones: {error}"),
        )
    }
}
