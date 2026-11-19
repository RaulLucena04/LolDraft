//! Estadísticas observadas importadas por el usuario.
//!
//! Este módulo es puro: define el esquema, valida, calcula shrinkage y ordena
//! los datos. No descarga nada, no escribe en SQLite y no conoce la UI. La app
//! importa un fichero local elegido por el usuario; el proyecto no raspa
//! Lolalytics ni redistribuye datos de terceros.
//!
//! Reglas de honestidad:
//! - ninguna fila se completa con promedios inventados: o viene en el fichero o
//!   se queda fuera con su motivo;
//! - el winrate nunca se usa crudo: se aplica shrinkage hacia el 50 % con un
//!   prior explícito, y la confianza depende del tamaño de muestra;
//! - solo queue y profesional viven en datasets separados (`queue`), nunca en
//!   una cifra mezclada sin etiqueta.

use loldraft_domain::{ChampionKey, Role};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Versión del documento de importación que entiende este código.
pub const STATS_SCHEMA_VERSION: &str = "1";

/// Partidas del prior en el shrinkage. Con `PRIOR_GAMES` partidas la
/// confianza es 0.5; con el doble, 2/3. Es una decisión conservadora: una
/// muestra pequeña no puede mover mucho el winrate ajustado.
pub const PRIOR_GAMES: f32 = 200.0;

/// Winrate del prior: no se sabe nada del campeón, así que se parte del 50 %.
pub const PRIOR_WINRATE: f32 = 0.5;

/// Mínimo de partidas para usar la estadística en una señal del motor. Por
/// debajo, el dato se guarda y se muestra con su `n`, pero no se puntúa.
pub const MIN_GAMES_FOR_POWER: u32 = 200;

/// Límite defensivo para ficheros importados a mano.
pub const MAX_ROWS: usize = 5000;

/// Cola del dataset. Pro y SoloQ no se mezclan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatsQueue {
    Soloq,
    Pro,
}

impl StatsQueue {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soloq => "soloq",
            Self::Pro => "pro",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Soloq => "SoloQ",
            Self::Pro => "Profesional",
        }
    }

    pub fn from_code(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "soloq" | "solo_q" | "solo-queue" => Some(Self::Soloq),
            "pro" | "professional" | "profesional" => Some(Self::Pro),
            _ => None,
        }
    }
}

/// Una fila observada de campeón y rol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChampionStatRow {
    pub champion: ChampionKey,
    pub role: Role,
    pub games: u32,
    pub wins: u32,
    /// Presencia en la cola (0..1), si la fuente la trae.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick_rate: Option<f32>,
    /// Baneos / partidas (0..1), si la fuente la trae.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ban_rate: Option<f32>,
}

impl ChampionStatRow {
    /// Winrate observado sin ajustar. `None` si no hay partidas.
    pub fn raw_winrate(&self) -> Option<f32> {
        (self.games > 0).then(|| self.wins as f32 / self.games as f32)
    }

    /// Winrate con shrinkage hacia el 50 %. Es el único valor que debe usar el
    /// motor para puntuar: el crudo se muestra como dato, no como señal.
    pub fn shrunk_winrate(&self) -> f32 {
        let games = self.games as f32;
        (self.wins as f32 + PRIOR_GAMES * PRIOR_WINRATE) / (games + PRIOR_GAMES)
    }

    /// Confianza en la muestra, `games / (games + PRIOR_GAMES)`, en 0..1.
    pub fn confidence(&self) -> f32 {
        let games = self.games as f32;
        games / (games + PRIOR_GAMES)
    }

    /// `true` si la muestra basta para alimentar `power`.
    pub fn has_enough_games(&self) -> bool {
        self.games >= MIN_GAMES_FOR_POWER
    }
}

/// Dataset de estadísticas observadas. Se importa entero o se rechaza.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedStats {
    /// Versión del esquema del documento, no del dataset.
    pub schema: String,
    pub id: String,
    pub version: String,
    pub source: String,
    pub license: String,
    pub queue: StatsQueue,
    pub patch: String,
    pub rows: Vec<ChampionStatRow>,
}

impl ObservedStats {
    /// Parsea un documento JSON estricto. Un campo desconocido, una fila
    /// duplicada o un valor fuera de rango invalidan el dataset completo.
    pub fn from_json(json: &str) -> Result<Self, StatsError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> Result<(), StatsError> {
        if self.schema != STATS_SCHEMA_VERSION {
            return Err(StatsError::UnsupportedSchema {
                found: self.schema.clone(),
                expected: STATS_SCHEMA_VERSION,
            });
        }
        require_text("id", &self.id)?;
        require_text("version", &self.version)?;
        require_text("source", &self.source)?;
        require_text("license", &self.license)?;
        validate_patch(&self.patch)?;
        if self.rows.is_empty() {
            return Err(StatsError::NoRows);
        }
        if self.rows.len() > MAX_ROWS {
            return Err(StatsError::TooManyRows {
                rows: self.rows.len(),
            });
        }

        let mut seen = BTreeSet::new();
        for row in &self.rows {
            validate_row(row, None)?;
            if !seen.insert((row.champion, row.role)) {
                return Err(StatsError::DuplicateRow {
                    champion: row.champion,
                    role: row.role,
                });
            }
        }
        Ok(())
    }

    /// Fila de un campeón en un rol concreto.
    pub fn row(&self, champion: ChampionKey, role: Role) -> Option<&ChampionStatRow> {
        self.rows
            .iter()
            .find(|row| row.champion == champion && row.role == role)
    }

    /// Percentil de winrate ajustado **dentro del mismo rol**, entre las filas
    /// con muestra suficiente. El percentil evita comparar roles con bases
    /// distintas y no inventa un baseline externo: sale del propio dataset.
    ///
    /// Empates comparten la media de sus posiciones. Si no hay dos filas
    /// comparables en el rol, no hay señal (`None`).
    pub fn power_percentiles(&self) -> BTreeMap<(ChampionKey, Role), f32> {
        let mut result = BTreeMap::new();
        for role in Role::ALL {
            let mut eligible: Vec<&ChampionStatRow> = self
                .rows
                .iter()
                .filter(|row| row.role == role && row.has_enough_games())
                .collect();
            if eligible.len() < 2 {
                continue;
            }
            eligible.sort_by(|left, right| {
                left.shrunk_winrate()
                    .partial_cmp(&right.shrunk_winrate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let total = eligible.len();
            let mut index = 0;
            while index < total {
                let mut end = index + 1;
                while end < total
                    && (eligible[end].shrunk_winrate() - eligible[index].shrunk_winrate()).abs()
                        < f32::EPSILON
                {
                    end += 1;
                }
                // Posiciones 0..total-1; el percentil va de 0 a 1.
                let average_rank = (index + end - 1) as f32 / 2.0;
                let percentile = if total > 1 {
                    average_rank / (total - 1) as f32
                } else {
                    0.5
                };
                for row in &eligible[index..end] {
                    result.insert((row.champion, row.role), percentile);
                }
                index = end;
            }
        }
        result
    }
}

/// Metadatos que el CSV no puede contener y que el usuario aporta al importar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsCsvMeta {
    pub id: String,
    pub version: String,
    pub source: String,
    pub license: String,
    pub queue: StatsQueue,
    pub patch: String,
}

impl StatsCsvMeta {
    fn into_document(self, rows: Vec<ChampionStatRow>) -> ObservedStats {
        ObservedStats {
            schema: STATS_SCHEMA_VERSION.to_string(),
            id: self.id,
            version: self.version,
            source: self.source,
            license: self.license,
            queue: self.queue,
            patch: self.patch,
            rows,
        }
    }
}

/// Fila que no se pudo importar y por qué.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardedRow {
    /// Número de línea del fichero (1-based). `None` en documentos JSON.
    pub line: Option<usize>,
    pub reason: String,
}

/// Resultado de importar un CSV: dataset válido + filas en cuarentena.
#[derive(Debug, Clone, PartialEq)]
pub struct StatsCsvImport {
    pub stats: ObservedStats,
    pub discarded: Vec<DiscardedRow>,
}

impl StatsCsvImport {
    pub fn summary_es(&self) -> String {
        let mut text = format!(
            "{} filas importadas ({} · parche {}).",
            self.stats.rows.len(),
            self.stats.queue.label_es(),
            self.stats.patch
        );
        if !self.discarded.is_empty() {
            text.push_str(&format!(
                " {} filas descartadas; revisa el detalle.",
                self.discarded.len()
            ));
        }
        text
    }
}

/// Importa un CSV con cabecera. Columnas obligatorias: `champion` (o
/// `champion_key`), `role`, `games` y `wins`. Opcionales: `pick_rate` y
/// `ban_rate`. El orden de las columnas es libre.
///
/// Las filas inválidas se descartan con su motivo en lugar de tumbar el
/// fichero entero. Si no queda ninguna fila válida, el import falla. Las
/// comillas no se aceptan: el formato es deliberadamente simple y comprobable.
pub fn parse_stats_csv(text: &str, meta: StatsCsvMeta) -> Result<StatsCsvImport, StatsError> {
    let mut meta = meta;

    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line));
    let mut header: Option<Vec<Column>> = None;
    let mut rows = Vec::new();
    let mut discarded = Vec::new();
    let mut seen = BTreeSet::new();

    for (line_number, line) in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix('#') {
            // Metadatos en comentarios antes de la cabecera. Así un CSV puede
            // ser autodescriptivo sin un formulario aparte.
            if header.is_none() {
                apply_csv_comment(&mut meta, comment, line_number)?;
            }
            continue;
        }
        if trimmed.contains('"') {
            return Err(StatsError::QuotedCsvField { line: line_number });
        }
        let fields: Vec<&str> = trimmed.split(',').map(str::trim).collect();
        match &header {
            None => {
                header = Some(parse_header(&fields, line_number)?);
            }
            Some(columns) => {
                if fields.len() != columns.len() {
                    discarded.push(DiscardedRow {
                        line: Some(line_number),
                        reason: format!(
                            "se esperaban {} columnas y hay {}",
                            columns.len(),
                            fields.len()
                        ),
                    });
                    continue;
                }
                match parse_csv_row(columns, &fields, line_number) {
                    Ok(row) => {
                        if !seen.insert((row.champion, row.role)) {
                            discarded.push(DiscardedRow {
                                line: Some(line_number),
                                reason: format!(
                                    "duplicado: campeón {} · rol {}",
                                    row.champion.0,
                                    row.role.as_str()
                                ),
                            });
                            continue;
                        }
                        rows.push(row);
                    }
                    Err(reason) => discarded.push(DiscardedRow {
                        line: Some(line_number),
                        reason,
                    }),
                }
            }
        }
    }

    if header.is_none() {
        return Err(StatsError::EmptyCsv);
    }
    if rows.is_empty() {
        return Err(StatsError::NoValidRows {
            discarded: discarded.len(),
        });
    }
    if rows.len() > MAX_ROWS {
        return Err(StatsError::TooManyRows { rows: rows.len() });
    }

    require_text("id", &meta.id)?;
    require_text("version", &meta.version)?;
    require_text("source", &meta.source)?;
    require_text("license", &meta.license)?;
    validate_patch(&meta.patch)?;

    let stats = meta.into_document(rows);
    stats.validate()?;
    Ok(StatsCsvImport { stats, discarded })
}

/// Aplica una línea `# clave: valor` a los metadatos. Los comentarios sin
/// `clave: valor` se ignoran: son notas para humanos.
fn apply_csv_comment(
    meta: &mut StatsCsvMeta,
    comment: &str,
    line: usize,
) -> Result<(), StatsError> {
    let Some((key, value)) = comment.split_once(':') else {
        return Ok(());
    };
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    match key.as_str() {
        "id" => meta.id = value.to_string(),
        "version" => meta.version = value.to_string(),
        "source" | "fuente" => meta.source = value.to_string(),
        "license" | "licencia" => meta.license = value.to_string(),
        "patch" | "parche" => meta.patch = value.to_string(),
        "queue" | "cola" => {
            meta.queue = StatsQueue::from_code(value).ok_or_else(|| StatsError::InvalidQueue {
                line,
                value: value.to_string(),
            })?;
        }
        // Otras claves son notas libres.
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Champion,
    Role,
    Games,
    Wins,
    PickRate,
    BanRate,
}

fn parse_header(fields: &[&str], line: usize) -> Result<Vec<Column>, StatsError> {
    let mut columns = Vec::with_capacity(fields.len());
    for field in fields {
        let normalized = field.to_ascii_lowercase();
        let column = match normalized.as_str() {
            "champion" | "champion_key" | "championkey" => Column::Champion,
            "role" | "position" => Column::Role,
            "games" | "n" => Column::Games,
            "wins" | "w" => Column::Wins,
            "pick_rate" | "pickrate" => Column::PickRate,
            "ban_rate" | "banrate" => Column::BanRate,
            other => {
                return Err(StatsError::UnknownCsvColumn {
                    line,
                    column: other.to_string(),
                })
            }
        };
        if columns.contains(&column) {
            return Err(StatsError::DuplicateCsvColumn {
                line,
                column: field.to_string(),
            });
        }
        columns.push(column);
    }
    for required in [Column::Champion, Column::Role, Column::Games, Column::Wins] {
        if !columns.contains(&required) {
            return Err(StatsError::MissingCsvColumn {
                column: column_name(required).to_string(),
            });
        }
    }
    Ok(columns)
}

fn column_name(column: Column) -> &'static str {
    match column {
        Column::Champion => "champion",
        Column::Role => "role",
        Column::Games => "games",
        Column::Wins => "wins",
        Column::PickRate => "pick_rate",
        Column::BanRate => "ban_rate",
    }
}

fn parse_csv_row(
    columns: &[Column],
    fields: &[&str],
    line: usize,
) -> Result<ChampionStatRow, String> {
    let mut champion = None;
    let mut role = None;
    let mut games = None;
    let mut wins = None;
    let mut pick_rate = None;
    let mut ban_rate = None;

    for (column, field) in columns.iter().zip(fields) {
        match column {
            Column::Champion => {
                champion = Some(parse_u16(field, "champion", line)?);
            }
            Column::Role => {
                role = Some(
                    Role::from_code(&field.to_ascii_lowercase())
                        .ok_or_else(|| format!("rol desconocido `{field}`"))?,
                );
            }
            Column::Games => {
                games = Some(parse_u32(field, "games", line)?);
            }
            Column::Wins => {
                wins = Some(parse_u32(field, "wins", line)?);
            }
            Column::PickRate => {
                pick_rate = parse_optional_rate(field, "pick_rate", line)?;
            }
            Column::BanRate => {
                ban_rate = parse_optional_rate(field, "ban_rate", line)?;
            }
        }
    }

    let row = ChampionStatRow {
        champion: ChampionKey(champion.ok_or_else(|| "falta la columna `champion`".to_string())?),
        role: role.ok_or_else(|| "falta la columna `role`".to_string())?,
        games: games.ok_or_else(|| "falta la columna `games`".to_string())?,
        wins: wins.ok_or_else(|| "falta la columna `wins`".to_string())?,
        pick_rate,
        ban_rate,
    };
    validate_row(&row, Some(line)).map_err(|error| error.to_string())?;
    Ok(row)
}

fn parse_u16(field: &str, name: &str, _line: usize) -> Result<u16, String> {
    field
        .parse::<u16>()
        .map_err(|_| format!("`{name}` no es un entero válido: `{field}`"))
}

fn parse_u32(field: &str, name: &str, line: usize) -> Result<u32, String> {
    let value = field
        .parse::<u64>()
        .map_err(|_| format!("`{name}` no es un entero válido: `{field}`"))?;
    u32::try_from(value).map_err(|_| format!("`{name}` fuera de rango en la línea {line}: {value}"))
}

fn parse_optional_rate(field: &str, name: &str, line: usize) -> Result<Option<f32>, String> {
    if field.is_empty() {
        return Ok(None);
    }
    let value = field
        .parse::<f32>()
        .map_err(|_| format!("`{name}` no es un número válido: `{field}`"))?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!(
            "`{name}` fuera de 0..1 en la línea {line}: {value}"
        ));
    }
    Ok(Some(value))
}

fn validate_row(row: &ChampionStatRow, line: Option<usize>) -> Result<(), StatsError> {
    if row.champion.0 == 0 {
        return Err(StatsError::InvalidChampionKey { line });
    }
    if row.games == 0 {
        return Err(StatsError::ZeroGames { line });
    }
    if row.wins > row.games {
        return Err(StatsError::WinsAboveGames {
            line,
            games: row.games,
            wins: row.wins,
        });
    }
    for (field, value) in [("pick_rate", row.pick_rate), ("ban_rate", row.ban_rate)] {
        if let Some(value) = value {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(StatsError::InvalidRate { line, field, value });
            }
        }
    }
    Ok(())
}

fn require_text(field: &'static str, value: &str) -> Result<(), StatsError> {
    if value.trim().is_empty() {
        Err(StatsError::EmptyField { field })
    } else {
        Ok(())
    }
}

fn validate_patch(patch: &str) -> Result<(), StatsError> {
    let parts: Vec<&str> = patch.trim().split('.').collect();
    let valid = matches!(parts.len(), 2 | 3)
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    if valid {
        Ok(())
    } else {
        Err(StatsError::InvalidPatch {
            patch: patch.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StatsError {
    #[error("no se pudo leer el documento de estadísticas: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("esquema `{found}` no soportado; este código entiende `{expected}`")]
    UnsupportedSchema {
        found: String,
        expected: &'static str,
    },
    #[error("campo vacío `{field}` en el dataset de estadísticas")]
    EmptyField { field: &'static str },
    #[error("parche no reconocible: `{patch}` (se espera major.minor o major.minor.patch)")]
    InvalidPatch { patch: String },
    #[error("el dataset no tiene filas")]
    NoRows,
    #[error("el dataset tiene {rows} filas y el límite es {MAX_ROWS}")]
    TooManyRows { rows: usize },
    #[error("la clave de campeón 0 no es válida")]
    InvalidChampionKey { line: Option<usize> },
    #[error("fila de campeón con 0 partidas")]
    ZeroGames { line: Option<usize> },
    #[error("la fila declara {wins} victorias con solo {games} partidas")]
    WinsAboveGames {
        line: Option<usize>,
        games: u32,
        wins: u32,
    },
    #[error("valor fuera de 0..1 en `{field}`: {value}")]
    InvalidRate {
        line: Option<usize>,
        field: &'static str,
        value: f32,
    },
    #[error("el campeón {champion} · rol {role:?} aparece más de una vez")]
    DuplicateRow { champion: ChampionKey, role: Role },
    #[error("el CSV está vacío: falta la cabecera")]
    EmptyCsv,
    #[error("cola desconocida en la línea {line}: `{value}` (usa soloq o pro)")]
    InvalidQueue { line: usize, value: String },
    #[error("columna desconocida en la cabecera (línea {line}): `{column}`")]
    UnknownCsvColumn { line: usize, column: String },
    #[error("columna repetida en la cabecera (línea {line}): `{column}`")]
    DuplicateCsvColumn { line: usize, column: String },
    #[error("falta la columna obligatoria `{column}` en la cabecera")]
    MissingCsvColumn { column: String },
    #[error("las comillas no están soportadas en el CSV (línea {line})")]
    QuotedCsvField { line: usize },
    #[error("ninguna fila válida; {discarded} descartadas. Revisa los motivos")]
    NoValidRows { discarded: usize },
}

impl StatsError {
    /// Línea del fichero cuando el error viene de una fila concreta.
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::InvalidChampionKey { line }
            | Self::ZeroGames { line }
            | Self::WinsAboveGames { line, .. }
            | Self::InvalidRate { line, .. } => *line,
            Self::UnknownCsvColumn { line, .. }
            | Self::DuplicateCsvColumn { line, .. }
            | Self::InvalidQueue { line, .. }
            | Self::QuotedCsvField { line } => Some(*line),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> StatsCsvMeta {
        StatsCsvMeta {
            id: "manual-soloq-16.22".to_string(),
            version: "1".to_string(),
            source: "Importación manual".to_string(),
            license: "Uso personal".to_string(),
            queue: StatsQueue::Soloq,
            patch: "16.22".to_string(),
        }
    }

    fn row(champion: u16, role: Role, games: u32, wins: u32) -> ChampionStatRow {
        ChampionStatRow {
            champion: ChampionKey(champion),
            role,
            games,
            wins,
            pick_rate: None,
            ban_rate: None,
        }
    }

    #[test]
    fn shrinkage_moves_small_samples_toward_fifty_percent() {
        let small = row(64, Role::Jungle, 10, 10);
        assert!((small.raw_winrate().unwrap() - 1.0).abs() < 1e-6);
        assert!((small.shrunk_winrate() - (10.0 + 100.0) / 210.0).abs() < 1e-6);
        assert!(small.shrunk_winrate() < 0.55);
        assert!((small.confidence() - 10.0 / 210.0).abs() < 1e-6);
        assert!(!small.has_enough_games());

        let large = row(64, Role::Jungle, 2000, 1100);
        assert!((large.shrunk_winrate() - (1100.0 + 100.0) / 2200.0).abs() < 1e-6);
        assert!(large.shrunk_winrate() > 0.5);
        assert!(large.has_enough_games());
    }

    #[test]
    fn a_row_without_games_has_no_raw_winrate() {
        let empty = row(64, Role::Jungle, 0, 0);
        assert_eq!(empty.raw_winrate(), None);
        assert_eq!(empty.shrunk_winrate(), PRIOR_WINRATE);
        assert_eq!(empty.confidence(), 0.0);
    }

    #[test]
    fn power_uses_percentiles_within_the_same_role() {
        let stats = ObservedStats {
            schema: STATS_SCHEMA_VERSION.to_string(),
            id: "test".to_string(),
            version: "1".to_string(),
            source: "test".to_string(),
            license: "test".to_string(),
            queue: StatsQueue::Soloq,
            patch: "16.22".to_string(),
            rows: vec![
                row(1, Role::Jungle, 1000, 450),
                row(2, Role::Jungle, 1000, 500),
                row(3, Role::Jungle, 1000, 550),
                // Muestra insuficiente: queda fuera del percentil.
                row(4, Role::Jungle, 50, 45),
                // Otro rol: no se compara con jungla.
                row(5, Role::Top, 1000, 550),
            ],
        };
        stats.validate().unwrap();
        let power = stats.power_percentiles();
        assert_eq!(power.len(), 3);
        assert!((power[&(ChampionKey(1), Role::Jungle)] - 0.0).abs() < 1e-6);
        assert!((power[&(ChampionKey(2), Role::Jungle)] - 0.5).abs() < 1e-6);
        assert!((power[&(ChampionKey(3), Role::Jungle)] - 1.0).abs() < 1e-6);
        assert!(!power.contains_key(&(ChampionKey(4), Role::Jungle)));
        assert!(!power.contains_key(&(ChampionKey(5), Role::Top)));
    }

    #[test]
    fn ties_share_the_average_percentile() {
        let stats = ObservedStats {
            schema: STATS_SCHEMA_VERSION.to_string(),
            id: "test".to_string(),
            version: "1".to_string(),
            source: "test".to_string(),
            license: "test".to_string(),
            queue: StatsQueue::Soloq,
            patch: "16.22".to_string(),
            rows: vec![
                row(1, Role::Mid, 1000, 500),
                row(2, Role::Mid, 1000, 500),
                row(3, Role::Mid, 1000, 600),
            ],
        };
        let power = stats.power_percentiles();
        assert!((power[&(ChampionKey(1), Role::Mid)] - 0.25).abs() < 1e-6);
        assert!((power[&(ChampionKey(2), Role::Mid)] - 0.25).abs() < 1e-6);
        assert!((power[&(ChampionKey(3), Role::Mid)] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn csv_imports_and_reports_quarantined_rows() {
        let csv = "\u{feff}# exportado a mano\n\
champion,role,games,wins,pick_rate,ban_rate\n\
64,jungle,1200,620,0.08,0.12\n\
103,mid,900,470,,\n\
,mid,10,5,,\n\
266,top,10,11,,\n\
99,support,600,300,1.5,\n\
64,jungle,10,5,,\n\
103,mid,800,400,,\n\
999,mid,0,0,,\n\
1,mid,notanumber,0,,\n";
        let imported = parse_stats_csv(csv, meta()).expect("csv válido");
        assert_eq!(imported.stats.rows.len(), 2);
        assert_eq!(imported.discarded.len(), 7);
        let reasons: Vec<&str> = imported
            .discarded
            .iter()
            .map(|row| row.reason.as_str())
            .collect();
        assert!(reasons.iter().any(|reason| reason.contains("victorias")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("fuera de 0..1")));
        assert!(reasons.iter().any(|reason| reason.contains("duplicado")));
        assert!(reasons.iter().any(|reason| reason.contains("0 partidas")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("no es un entero")));
        assert!(imported.summary_es().contains("2 filas importadas"));
        assert!(imported.summary_es().contains("7 filas descartadas"));
    }

    #[test]
    fn csv_accepts_columns_in_any_order_and_blank_optional_rates() {
        let csv = "wins,games,role,champion\n620,1200,jungle,64\n";
        let imported = parse_stats_csv(csv, meta()).expect("csv válido");
        assert_eq!(imported.stats.rows.len(), 1);
        let row = &imported.stats.rows[0];
        assert_eq!(row.pick_rate, None);
        assert_eq!(row.ban_rate, None);
        assert!(imported.discarded.is_empty());
    }

    #[test]
    fn malformed_headers_and_quotes_are_rejected() {
        let missing = "champion,role,games\n64,jungle,10\n";
        assert!(matches!(
            parse_stats_csv(missing, meta()),
            Err(StatsError::MissingCsvColumn { .. })
        ));

        let unknown = "champion,role,games,wins,elo\n64,jungle,10,5,diamond\n";
        assert!(matches!(
            parse_stats_csv(unknown, meta()),
            Err(StatsError::UnknownCsvColumn { .. })
        ));

        let quoted = "champion,role,games,wins\n\"64\",jungle,10,5\n";
        assert!(matches!(
            parse_stats_csv(quoted, meta()),
            Err(StatsError::QuotedCsvField { .. })
        ));

        let duplicate = "champion,role,games,wins,games\n64,jungle,10,5,10\n";
        assert!(matches!(
            parse_stats_csv(duplicate, meta()),
            Err(StatsError::DuplicateCsvColumn { .. })
        ));
    }

    #[test]
    fn a_csv_without_valid_rows_is_an_error_not_an_empty_dataset() {
        let csv = "champion,role,games,wins\n0,jungle,10,5\n";
        assert!(matches!(
            parse_stats_csv(csv, meta()),
            Err(StatsError::NoValidRows { discarded: 1 })
        ));
    }

    #[test]
    fn csv_comments_can_describe_the_dataset() {
        let csv = "# id: comentado-16.22\n\
# version: 3\n\
# parche: 16.22\n\
# cola: pro\n\
# fuente: Exportación externa\n\
# licencia: CC0\n\
# nota libre para humanos\n\
champion,role,games,wins\n\
64,jungle,1200,620\n";
        let mut fallback = meta();
        fallback.id = "sin-metadatos".to_string();
        fallback.patch = String::new();

        let imported = parse_stats_csv(csv, fallback).expect("csv válido");
        assert_eq!(imported.stats.id, "comentado-16.22");
        assert_eq!(imported.stats.version, "3");
        assert_eq!(imported.stats.patch, "16.22");
        assert_eq!(imported.stats.queue, StatsQueue::Pro);
        assert_eq!(imported.stats.source, "Exportación externa");
        assert_eq!(imported.stats.license, "CC0");
    }

    #[test]
    fn an_unknown_queue_in_a_comment_is_an_error() {
        let csv = "# queue: aram\nchampion,role,games,wins\n64,jungle,10,5\n";
        assert!(matches!(
            parse_stats_csv(csv, meta()),
            Err(StatsError::InvalidQueue { .. })
        ));
    }

    #[test]
    fn csv_requires_metadata() {
        let mut invalid = meta();
        invalid.patch = "dieciseis".to_string();
        let csv = "champion,role,games,wins\n64,jungle,10,5\n";
        assert!(matches!(
            parse_stats_csv(csv, invalid),
            Err(StatsError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn json_dataset_roundtrips_and_rejects_strict_violations() {
        let document = r#"{
          "schema": "1",
          "id": "manual-soloq-16.22",
          "version": "1",
          "source": "Importación manual",
          "license": "Uso personal",
          "queue": "soloq",
          "patch": "16.22",
          "rows": [
            {"champion": 64, "role": "jungle", "games": 1200, "wins": 620},
            {"champion": 103, "role": "mid", "games": "900", "wins": 470}
          ]
        }"#;
        assert!(matches!(
            ObservedStats::from_json(document),
            Err(StatsError::Parse(_))
        ));

        let valid = document.replace("\"games\": \"900\"", "\"games\": 900");
        let stats = ObservedStats::from_json(&valid).expect("documento válido");
        assert_eq!(stats.rows.len(), 2);
        assert_eq!(stats.queue, StatsQueue::Soloq);

        let duplicate = valid.replace(
            "\"champion\": 103, \"role\": \"mid\"",
            "\"champion\": 64, \"role\": \"jungle\"",
        );
        assert!(matches!(
            ObservedStats::from_json(&duplicate),
            Err(StatsError::DuplicateRow { .. })
        ));

        let unknown = valid.replace("\"rows\"", "\"extra\": 1, \"rows\"");
        assert!(matches!(
            ObservedStats::from_json(&unknown),
            Err(StatsError::Parse(_))
        ));

        let wrong_schema = valid.replace("\"schema\": \"1\"", "\"schema\": \"2\"");
        assert!(matches!(
            ObservedStats::from_json(&wrong_schema),
            Err(StatsError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn queues_are_never_mixed_by_the_schema() {
        assert_eq!(StatsQueue::from_code("soloq"), Some(StatsQueue::Soloq));
        assert_eq!(StatsQueue::from_code("PRO"), Some(StatsQueue::Pro));
        assert_eq!(StatsQueue::from_code("aram"), None);
        assert_ne!(StatsQueue::Soloq.as_str(), StatsQueue::Pro.as_str());
    }
}
