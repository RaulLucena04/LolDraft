//! Dominio puro de LolDraft.
//!
//! No conoce Tauri, SQLite, HTTP ni React. Todas sus funciones son deterministas
//! y testables sin infraestructura.

pub mod champion;
pub mod draft;
pub mod error;
pub mod ids;

pub use champion::{Champion, ChampionCatalog, ChampionIdentity, RoleFit};
pub use draft::{
    assign_roles, assign_roles_avoiding, assign_roles_with_locks, DraftFormat, DraftSession,
    DraftState, DraftStep, RoleAssignment,
};
pub use error::{AssignmentError, DraftError};
pub use ids::{ActionKind, ChampionKey, DraftAction, Role, Side};
