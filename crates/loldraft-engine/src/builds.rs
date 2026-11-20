//! Builds y runas observadas por campeón y rol.
//!
//! Igual que las estadísticas, este módulo es puro: define el esquema, valida
//! el documento y elige la build con mejor winrate ajustado **dentro del
//! propio dataset**. No descarga builds de terceros ni inventa una build
//! cuando no hay datos: si no hay filas para el campeón, devuelve `None`.

use crate::stats::{StatsQueue, MIN_GAMES_FOR_POWER, PRIOR_GAMES, PRIOR_WINRATE};
use loldraft_domain::{ChampionKey, Role};
use serde::{Deserialize, Serialize};

/// Versión del esquema del documento, no del dataset.
pub const BUILDS_SCHEMA_VERSION: &str = "1";
/// Límite defensivo: un fichero enorme se rechaza antes de tocar SQLite.
pub const MAX_BUILD_ROWS: usize = 10_000;
/// Elementos máximos por lista de objetos (starter, core, situacionales).
pub const MAX_ITEMS_PER_LIST: usize = 20;
/// Longitud máxima de cualquier texto libre (nombre de build, objeto o runa).
pub const MAX_BUILD_TEXT: usize = 80;

/// Dataset de builds observadas. Se importa entero o se rechaza.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedBuilds {
    /// Versión del esquema del documento.
    pub schema: String,
    pub id: String,
    pub version: String,
    pub source: String,
    pub license: String,
    /// `soloq` o `pro`; nunca se mezclan.
    pub queue: StatsQueue,
    pub patch: String,
    pub rows: Vec<ChampionBuildRow>,
}

/// Una build concreta para un campeón en un rol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChampionBuildRow {
    pub champion: ChampionKey,
    pub role: Role,
    /// Nombre legible de la build ("Conqueror bruiser", "Engage support"…).
    pub name: String,
    #[serde(default)]
    pub starter_items: Vec<String>,
    /// Objetos núcleo. Obligatorio: una build sin núcleo no es una build.
    pub core_items: Vec<String>,
    #[serde(default)]
    pub situational_items: Vec<String>,
    pub runes: RunePage,
    /// Partidas observadas de esta build. 0 significa build curada sin muestra.
    #[serde(default)]
    pub games: u32,
    #[serde(default)]
    pub wins: u32,
}

/// Página de runas asociada a una build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunePage {
    pub primary_path: String,
    pub keystone: String,
    #[serde(default)]
    pub primary_runes: Vec<String>,
    pub secondary_path: String,
    #[serde(default)]
    pub secondary_runes: Vec<String>,
    #[serde(default)]
    pub shards: Vec<String>,
}

/// Por qué se eligió una build cuando hay varias filas para el campeón.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildBasis {
    /// La fila con mejor winrate ajustado entre las que superan el mínimo.
    ObservedWinrate,
    /// No hay muestra suficiente: se elige la fila con más partidas.
    SampleSize,
    /// Ninguna fila tiene partidas: build curada, sin pretensión estadística.
    OnlyBuild,
}

impl BuildBasis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ObservedWinrate => "observed_winrate",
            Self::SampleSize => "sample_size",
            Self::OnlyBuild => "only_build",
        }
    }
}

/// Procedencia del dataset que alimentó una build. La UI la muestra junto a
/// los objetos y runas; nunca se presenta una build sin decir de dónde sale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildsProvenance {
    pub id: String,
    pub version: String,
    pub queue: StatsQueue,
    pub patch: String,
    pub source: String,
    pub license: String,
    pub rows: usize,
    pub min_games: u32,
}

/// Build elegida y motivo de la elección.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildChoice<'a> {
    pub row: &'a ChampionBuildRow,
    pub basis: BuildBasis,
}

impl ChampionBuildRow {
    /// Winrate con el mismo shrinkage que las estadísticas. Solo se usa para
    /// comparar filas del dataset, nunca se muestra como winrate real.
    pub fn shrunk_winrate(&self) -> f32 {
        let games = self.games as f32;
        (self.wins as f32 + PRIOR_GAMES * PRIOR_WINRATE) / (games + PRIOR_GAMES)
    }

    /// Confianza en la muestra, `games / (games + PRIOR_GAMES)`.
    pub fn confidence(&self) -> f32 {
        let games = self.games as f32;
        games / (games + PRIOR_GAMES)
    }

    pub fn has_enough_games(&self) -> bool {
        self.games >= MIN_GAMES_FOR_POWER
    }
}

impl ObservedBuilds {
    pub fn from_json(json: &str) -> Result<Self, BuildsError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> Result<(), BuildsError> {
        if self.schema != BUILDS_SCHEMA_VERSION {
            return Err(BuildsError::UnsupportedSchema {
                found: self.schema.clone(),
                expected: BUILDS_SCHEMA_VERSION,
            });
        }
        require_dataset_text(&self.id, "id")?;
        require_dataset_text(&self.version, "version")?;
        require_dataset_text(&self.source, "source")?;
        require_dataset_text(&self.license, "license")?;
        validate_patch(&self.patch)?;
        if self.rows.is_empty() {
            return Err(BuildsError::NoRows);
        }
        if self.rows.len() > MAX_BUILD_ROWS {
            return Err(BuildsError::TooManyRows {
                rows: self.rows.len(),
            });
        }

        let mut seen = std::collections::BTreeSet::new();
        for (index, row) in self.rows.iter().enumerate() {
            let number = index + 1;
            validate_row(row, number)?;
            if !seen.insert((row.champion, row.role, row.name.trim().to_string())) {
                return Err(BuildsError::DuplicateRow {
                    row: number,
                    champion: row.champion,
                    role: row.role,
                    name: row.name.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn provenance(&self) -> BuildsProvenance {
        BuildsProvenance {
            id: self.id.clone(),
            version: self.version.clone(),
            queue: self.queue,
            patch: self.patch.clone(),
            source: self.source.clone(),
            license: self.license.clone(),
            rows: self.rows.len(),
            min_games: MIN_GAMES_FOR_POWER,
        }
    }

    /// Build elegida para un campeón, opcionalmente restringida a un rol.
    ///
    /// Orden de preferencia:
    /// 1. filas con muestra suficiente: mejor winrate ajustado;
    /// 2. si ninguna llega al mínimo, la fila con más partidas;
    /// 3. si ninguna tiene partidas, la primera del dataset (build curada).
    ///
    /// Los empates conservan el orden del fichero: el dataset manda.
    pub fn best_for(&self, champion: ChampionKey, role: Option<Role>) -> Option<BuildChoice<'_>> {
        let candidates: Vec<&ChampionBuildRow> = self
            .rows
            .iter()
            .filter(|row| row.champion == champion)
            .filter(|row| role.is_none_or(|wanted| row.role == wanted))
            .collect();
        if candidates.is_empty() {
            return None;
        }

        let eligible: Vec<&ChampionBuildRow> = candidates
            .iter()
            .copied()
            .filter(|row| row.has_enough_games())
            .collect();
        if !eligible.is_empty() {
            let best = pick_best(&eligible, |row| (row.shrunk_winrate(), row.games as f32));
            return Some(BuildChoice {
                row: best,
                basis: BuildBasis::ObservedWinrate,
            });
        }

        let sampled: Vec<&ChampionBuildRow> = candidates
            .iter()
            .copied()
            .filter(|row| row.games > 0)
            .collect();
        if !sampled.is_empty() {
            let best = pick_best(&sampled, |row| (row.games as f32, row.shrunk_winrate()));
            return Some(BuildChoice {
                row: best,
                basis: BuildBasis::SampleSize,
            });
        }

        Some(BuildChoice {
            row: candidates[0],
            basis: BuildBasis::OnlyBuild,
        })
    }
}

/// Máximo estable: ante puntuaciones iguales se queda con la primera fila.
fn pick_best<'a>(
    rows: &[&'a ChampionBuildRow],
    key: impl Fn(&ChampionBuildRow) -> (f32, f32),
) -> &'a ChampionBuildRow {
    let mut best = rows[0];
    let mut best_key = key(best);
    for row in &rows[1..] {
        let candidate = key(row);
        if candidate.0 > best_key.0 || (candidate.0 == best_key.0 && candidate.1 > best_key.1) {
            best = row;
            best_key = candidate;
        }
    }
    best
}

fn validate_row(row: &ChampionBuildRow, number: usize) -> Result<(), BuildsError> {
    if row.champion.0 == 0 {
        return Err(BuildsError::InvalidChampionKey { row: number });
    }
    require_text(&row.name, "name", number)?;
    if row.wins > row.games {
        return Err(BuildsError::WinsAboveGames {
            row: number,
            games: row.games,
            wins: row.wins,
        });
    }
    validate_items(&row.starter_items, "starter_items", number)?;
    validate_items(&row.core_items, "core_items", number)?;
    validate_items(&row.situational_items, "situational_items", number)?;
    if row.core_items.is_empty() {
        return Err(BuildsError::EmptyList {
            row: number,
            field: "core_items",
        });
    }

    require_text(&row.runes.primary_path, "runes.primary_path", number)?;
    require_text(&row.runes.keystone, "runes.keystone", number)?;
    require_text(&row.runes.secondary_path, "runes.secondary_path", number)?;
    if row
        .runes
        .primary_path
        .trim()
        .eq_ignore_ascii_case(row.runes.secondary_path.trim())
    {
        return Err(BuildsError::SameRunePaths { row: number });
    }
    validate_items(&row.runes.primary_runes, "runes.primary_runes", number)?;
    validate_items(&row.runes.secondary_runes, "runes.secondary_runes", number)?;
    validate_shards(&row.runes.shards, number)?;
    if row.runes.primary_runes.len() > 4 {
        return Err(BuildsError::TooManyItems {
            row: number,
            field: "runes.primary_runes",
            items: row.runes.primary_runes.len(),
            max: 4,
        });
    }
    if row.runes.secondary_runes.len() > 3 {
        return Err(BuildsError::TooManyItems {
            row: number,
            field: "runes.secondary_runes",
            items: row.runes.secondary_runes.len(),
            max: 3,
        });
    }
    if row.runes.shards.len() > 3 {
        return Err(BuildsError::TooManyItems {
            row: number,
            field: "runes.shards",
            items: row.runes.shards.len(),
            max: 3,
        });
    }
    Ok(())
}

fn validate_items(items: &[String], field: &'static str, row: usize) -> Result<(), BuildsError> {
    if items.len() > MAX_ITEMS_PER_LIST {
        return Err(BuildsError::TooManyItems {
            row,
            field,
            items: items.len(),
            max: MAX_ITEMS_PER_LIST,
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    for item in items {
        require_text(item, field, row)?;
        if !seen.insert(item.trim().to_ascii_lowercase()) {
            return Err(BuildsError::DuplicateItem {
                row,
                field,
                item: item.clone(),
            });
        }
    }
    Ok(())
}

fn validate_shards(items: &[String], row: usize) -> Result<(), BuildsError> {
    if items.len() > 3 {
        return Err(BuildsError::TooManyItems {
            row,
            field: "runes.shards",
            items: items.len(),
            max: 3,
        });
    }
    for item in items {
        require_text(item, "runes.shards", row)?;
    }
    Ok(())
}

fn require_text(value: &str, field: &'static str, row: usize) -> Result<(), BuildsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BuildsError::EmptyField { row, field });
    }
    if trimmed.chars().count() > MAX_BUILD_TEXT {
        return Err(BuildsError::TextTooLong {
            row,
            field,
            len: trimmed.chars().count(),
            max: MAX_BUILD_TEXT,
        });
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(BuildsError::ControlCharacter { row, field });
    }
    Ok(())
}

fn require_dataset_text(value: &str, field: &'static str) -> Result<(), BuildsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BuildsError::EmptyDatasetField { field });
    }
    if trimmed.chars().count() > MAX_BUILD_TEXT {
        return Err(BuildsError::DatasetTextTooLong {
            field,
            len: trimmed.chars().count(),
            max: MAX_BUILD_TEXT,
        });
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(BuildsError::DatasetControlCharacter { field });
    }
    Ok(())
}

fn validate_patch(patch: &str) -> Result<(), BuildsError> {
    let parts: Vec<&str> = patch.trim().split('.').collect();
    let valid = matches!(parts.len(), 2 | 3)
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    if valid {
        Ok(())
    } else {
        Err(BuildsError::InvalidPatch {
            patch: patch.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuildsError {
    #[error("no se pudo leer el documento de builds: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("esquema `{found}` no soportado; este código entiende `{expected}`")]
    UnsupportedSchema {
        found: String,
        expected: &'static str,
    },
    #[error("campo vacío `{field}` en la fila {row}")]
    EmptyField { row: usize, field: &'static str },
    #[error("campo vacío `{field}` en el documento de builds")]
    EmptyDatasetField { field: &'static str },
    #[error("texto demasiado largo en `{field}` del documento: {len} caracteres (máximo {max})")]
    DatasetTextTooLong {
        field: &'static str,
        len: usize,
        max: usize,
    },
    #[error("carácter de control en `{field}` del documento de builds")]
    DatasetControlCharacter { field: &'static str },
    #[error(
        "texto demasiado largo en `{field}` de la fila {row}: {len} caracteres (máximo {max})"
    )]
    TextTooLong {
        row: usize,
        field: &'static str,
        len: usize,
        max: usize,
    },
    #[error("carácter de control en `{field}` de la fila {row}")]
    ControlCharacter { row: usize, field: &'static str },
    #[error("la lista `{field}` de la fila {row} está vacía")]
    EmptyList { row: usize, field: &'static str },
    #[error("la lista `{field}` de la fila {row} tiene {items} elementos (máximo {max})")]
    TooManyItems {
        row: usize,
        field: &'static str,
        items: usize,
        max: usize,
    },
    #[error("elemento duplicado `{item}` en `{field}` de la fila {row}")]
    DuplicateItem {
        row: usize,
        field: &'static str,
        item: String,
    },
    #[error("parche no reconocible: `{patch}` (se espera major.minor o major.minor.patch)")]
    InvalidPatch { patch: String },
    #[error("la clave de campeón 0 no es válida en la fila {row}")]
    InvalidChampionKey { row: usize },
    #[error("la fila {row} declara {wins} victorias con solo {games} partidas")]
    WinsAboveGames { row: usize, games: u32, wins: u32 },
    #[error("el campeón {champion} · rol {role:?} repite la build `{name}` en la fila {row}")]
    DuplicateRow {
        row: usize,
        champion: ChampionKey,
        role: Role,
        name: String,
    },
    #[error("la fila {row} usa la misma ruta de runas como primaria y secundaria")]
    SameRunePaths { row: usize },
    #[error("el dataset no tiene builds")]
    NoRows,
    #[error("el dataset tiene {rows} builds y el límite es {MAX_BUILD_ROWS}")]
    TooManyRows { rows: usize },
}

impl BuildsError {
    /// Fila del documento cuando el error viene de una fila concreta.
    pub fn row(&self) -> Option<usize> {
        match self {
            Self::EmptyField { row, .. }
            | Self::TextTooLong { row, .. }
            | Self::ControlCharacter { row, .. }
            | Self::EmptyList { row, .. }
            | Self::TooManyItems { row, .. }
            | Self::DuplicateItem { row, .. }
            | Self::InvalidChampionKey { row }
            | Self::WinsAboveGames { row, .. }
            | Self::DuplicateRow { row, .. }
            | Self::SameRunePaths { row } => Some(*row),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loldraft_domain::ChampionKey;

    fn rune_page() -> RunePage {
        RunePage {
            primary_path: "Precision".into(),
            keystone: "Conqueror".into(),
            primary_runes: vec![
                "Triumph".into(),
                "Legend: Alacrity".into(),
                "Last Stand".into(),
            ],
            secondary_path: "Inspiration".into(),
            secondary_runes: vec!["Magical Footwear".into(), "Cosmic Insight".into()],
            shards: vec![
                "Adaptive Force".into(),
                "Adaptive Force".into(),
                "Health Scaling".into(),
            ],
        }
    }

    fn build(name: &str, games: u32, wins: u32) -> ChampionBuildRow {
        ChampionBuildRow {
            champion: ChampionKey(64),
            role: Role::Jungle,
            name: name.into(),
            starter_items: vec!["Doran's Blade".into(), "Health Potion".into()],
            core_items: vec!["Eclipse".into(), "Sundered Sky".into()],
            situational_items: vec!["Death's Dance".into()],
            runes: rune_page(),
            games,
            wins,
        }
    }

    fn dataset(rows: Vec<ChampionBuildRow>) -> ObservedBuilds {
        ObservedBuilds {
            schema: BUILDS_SCHEMA_VERSION.into(),
            id: "builds-soloq".into(),
            version: "1".into(),
            source: "Prueba".into(),
            license: "Uso personal".into(),
            queue: StatsQueue::Soloq,
            patch: "16.22".into(),
            rows,
        }
    }

    #[test]
    fn json_roundtrips_and_rejects_unknown_fields() {
        let original = dataset(vec![build("Conqueror bruiser", 1200, 620)]);
        let json = serde_json::to_string(&original).unwrap();
        let parsed = ObservedBuilds::from_json(&json).unwrap();
        assert_eq!(parsed, original);

        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["extra"] = serde_json::json!(true);
        assert!(ObservedBuilds::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn a_core_build_is_required_and_paths_cannot_repeat() {
        let mut row = build("Conqueror bruiser", 0, 0);
        row.core_items.clear();
        assert!(matches!(
            dataset(vec![row]).validate(),
            Err(BuildsError::EmptyList {
                field: "core_items",
                ..
            })
        ));

        let mut row = build("Conqueror bruiser", 0, 0);
        row.runes.secondary_path = "Precision".into();
        assert!(matches!(
            dataset(vec![row]).validate(),
            Err(BuildsError::SameRunePaths { .. })
        ));
    }

    #[test]
    fn duplicate_champion_role_and_name_is_rejected() {
        let error = dataset(vec![
            build("Conqueror bruiser", 0, 0),
            build("Conqueror bruiser", 0, 0),
        ])
        .validate()
        .unwrap_err();
        assert!(matches!(error, BuildsError::DuplicateRow { row: 2, .. }));
    }

    #[test]
    fn wins_cannot_exceed_games() {
        let error = dataset(vec![build("Conqueror bruiser", 10, 11)])
            .validate()
            .unwrap_err();
        assert!(matches!(
            error,
            BuildsError::WinsAboveGames {
                games: 10,
                wins: 11,
                ..
            }
        ));
    }

    #[test]
    fn best_build_is_the_best_adjusted_winrate_with_enough_games() {
        let data = dataset(vec![
            build("A", 1000, 500),
            build("B", 1000, 560),
            build("C", 100, 90),
        ]);
        let choice = data.best_for(ChampionKey(64), Some(Role::Jungle)).unwrap();
        assert_eq!(choice.row.name, "B");
        assert_eq!(choice.basis, BuildBasis::ObservedWinrate);
    }

    #[test]
    fn without_minimum_sample_the_most_played_build_wins() {
        let data = dataset(vec![
            build("A", 30, 20),
            build("B", 120, 60),
            build("C", 0, 0),
        ]);
        let choice = data.best_for(ChampionKey(64), Some(Role::Jungle)).unwrap();
        assert_eq!(choice.row.name, "B");
        assert_eq!(choice.basis, BuildBasis::SampleSize);
    }

    #[test]
    fn a_curated_build_without_games_is_labelled_as_such() {
        let data = dataset(vec![build("Curada", 0, 0)]);
        let choice = data.best_for(ChampionKey(64), Some(Role::Jungle)).unwrap();
        assert_eq!(choice.row.name, "Curada");
        assert_eq!(choice.basis, BuildBasis::OnlyBuild);
    }

    #[test]
    fn a_different_role_or_champion_has_no_build() {
        let data = dataset(vec![build("A", 1000, 500)]);
        assert!(data.best_for(ChampionKey(64), Some(Role::Top)).is_none());
        assert!(data.best_for(ChampionKey(1), Some(Role::Jungle)).is_none());
        // Sin rol explícito se busca en cualquier rol del campeón.
        assert!(data.best_for(ChampionKey(64), None).is_some());
    }

    #[test]
    fn the_shipped_sample_is_valid() {
        let json = include_str!("../../../data/samples/builds-soloq.example.json");
        let sample = ObservedBuilds::from_json(json).unwrap();
        assert_eq!(sample.schema, BUILDS_SCHEMA_VERSION);
        assert_eq!(sample.queue, StatsQueue::Soloq);
        assert!(sample.rows.iter().all(|row| row.games == 0));
        assert!(sample
            .best_for(ChampionKey(64), Some(Role::Jungle))
            .is_some());
    }

    #[test]
    fn ties_keep_the_file_order() {
        let data = dataset(vec![
            build("Primera", 1000, 550),
            build("Segunda", 1000, 550),
        ]);
        let choice = data.best_for(ChampionKey(64), Some(Role::Jungle)).unwrap();
        assert_eq!(choice.row.name, "Primera");
    }
}
