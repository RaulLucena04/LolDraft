use crate::ids::{ActionKind, ChampionKey, Side};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DraftError {
    #[error("El draft ya está completo")]
    Complete,

    #[error("Acción fuera de turno: se esperaba {expected_side:?} {expected_kind:?}, se recibió {actual_side:?} {actual_kind:?}")]
    WrongStep {
        expected_side: Side,
        expected_kind: ActionKind,
        actual_side: Side,
        actual_kind: ActionKind,
    },

    #[error("El campeón {0} ya está usado en este draft")]
    ChampionAlreadyUsed(ChampionKey),

    #[error("Campeón desconocido: {0}")]
    UnknownChampion(ChampionKey),

    #[error("La acción no se puede aplicar porque no hay un paso disponible")]
    NoNextStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssignmentError {
    #[error("No hay roles disponibles para la composición")]
    NoRoles,
}
