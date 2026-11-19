//! Persistencia local, semillas e importación.
//!
//! `loldraft-data` no conoce Tauri ni React.

pub mod db;
pub mod error;
pub mod profiles;
pub mod seed;
pub mod stats;

pub use db::{now_ms, Database, SessionRecord, SessionSummary};
pub use error::DataError;
pub use profiles::{ActiveProfileSet, ProfileSetSummary};
pub use seed::{
    default_seed, default_seed_json, parse_seed, ChampionIdentity, ChampionSeedFile, SeedChampion,
    SeedDataset,
};
pub use stats::StatsDatasetSummary;
