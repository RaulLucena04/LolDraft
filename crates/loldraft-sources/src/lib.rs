//! Adaptadores de fuentes externas. No escriben en SQLite: devuelven datos
//! normalizados para que la capa de aplicación los valide y persista.

pub mod ddragon;
pub mod error;

pub use ddragon::{identities_from_champion_json, DdragonClient};
pub use error::SourceError;
