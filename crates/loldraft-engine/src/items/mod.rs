//! Objetos y runas recomendados por función y matchup.
//!
//! Este módulo no usa estadísticas de builds: el catálogo describe **qué hace**
//! cada objeto y cada página de runas, y la recomendación cruza esa función con
//! las señales del rival y con el perfil del campeón propio. Es un índice de
//! encaje estratégico, no una probabilidad de victoria ni una build observada.
//!
//! Cuando falta el perfil del campeón se usan sus tags de Riot como respaldo,
//! siempre marcados como tales. Si tampoco hay tags, no se inventa encaje.

#[cfg(test)]
mod tests;

use crate::analysis::{Coverage, TeamAnalysis};
use crate::profile::{ChampionProfile, Provenance};
use crate::taxonomy::{ArchetypeId, TraitId};
use loldraft_domain::{ChampionKey, Role};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::OnceLock;

/// Versión del esquema del documento del catálogo.
pub const ITEM_CATALOG_SCHEMA: &str = "2";
/// Versión del catálogo curado incluido en el binario.
pub const ITEM_CATALOG_VERSION: &str = "2.0.0";
/// Versión de las reglas que puntúan objetos y runas.
pub const LOADOUT_RULES_VERSION: &str = "2.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemCategory {
    Damage,
    Defense,
    Utility,
}

impl ItemCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Defense => "defense",
            Self::Utility => "utility",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Damage => "Daño",
            Self::Defense => "Defensa",
            Self::Utility => "Utilidad",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSlot {
    Starter,
    Boots,
    Item,
}

/// Afinidad funcional que un objeto o página de runas necesita del campeón.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitTag {
    Tank,
    Frontline,
    Bruiser,
    Assassin,
    Burst,
    Mage,
    Marksman,
    Support,
    Enchanter,
    Engage,
    Poke,
    Sustain,
    AutoAttack,
    Ad,
    Ap,
    Mana,
    Crit,
    OnHit,
    Mobility,
    Durability,
    EarlyGame,
    LateGame,
    Utility,
    HealShield,
    AntiTank,
    Dps,
    Splitpush,
    Catch,
}

impl FitTag {
    pub const ALL: [FitTag; 28] = [
        FitTag::Tank,
        FitTag::Frontline,
        FitTag::Bruiser,
        FitTag::Assassin,
        FitTag::Burst,
        FitTag::Mage,
        FitTag::Marksman,
        FitTag::Support,
        FitTag::Enchanter,
        FitTag::Engage,
        FitTag::Poke,
        FitTag::Sustain,
        FitTag::AutoAttack,
        FitTag::Ad,
        FitTag::Ap,
        FitTag::Mana,
        FitTag::Crit,
        FitTag::OnHit,
        FitTag::Mobility,
        FitTag::Durability,
        FitTag::EarlyGame,
        FitTag::LateGame,
        FitTag::Utility,
        FitTag::HealShield,
        FitTag::AntiTank,
        FitTag::Dps,
        FitTag::Splitpush,
        FitTag::Catch,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tank => "tank",
            Self::Frontline => "frontline",
            Self::Bruiser => "bruiser",
            Self::Assassin => "assassin",
            Self::Burst => "burst",
            Self::Mage => "mage",
            Self::Marksman => "marksman",
            Self::Support => "support",
            Self::Enchanter => "enchanter",
            Self::Engage => "engage",
            Self::Poke => "poke",
            Self::Sustain => "sustain",
            Self::AutoAttack => "auto_attack",
            Self::Ad => "ad",
            Self::Ap => "ap",
            Self::Mana => "mana",
            Self::Crit => "crit",
            Self::OnHit => "on_hit",
            Self::Mobility => "mobility",
            Self::Durability => "durability",
            Self::EarlyGame => "early_game",
            Self::LateGame => "late_game",
            Self::Utility => "utility",
            Self::HealShield => "heal_shield",
            Self::AntiTank => "anti_tank",
            Self::Dps => "dps",
            Self::Splitpush => "splitpush",
            Self::Catch => "catch",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Tank => "tanque",
            Self::Frontline => "frontline",
            Self::Bruiser => "bruiser",
            Self::Assassin => "assassin",
            Self::Burst => "burst",
            Self::Mage => "mago",
            Self::Marksman => "tirador",
            Self::Support => "support",
            Self::Enchanter => "enchanter",
            Self::Engage => "engage",
            Self::Poke => "poke",
            Self::Sustain => "sustain",
            Self::AutoAttack => "ataque básico",
            Self::Ad => "daño físico",
            Self::Ap => "daño mágico",
            Self::Mana => "maná",
            Self::Crit => "crítico",
            Self::OnHit => "efectos on-hit",
            Self::Mobility => "movilidad",
            Self::Durability => "durabilidad",
            Self::EarlyGame => "juego temprano",
            Self::LateGame => "escalado",
            Self::Utility => "utilidad",
            Self::HealShield => "curación o escudos",
            Self::AntiTank => "anti-tanque",
            Self::Dps => "DPS sostenido",
            Self::Splitpush => "splitpush",
            Self::Catch => "catch",
        }
    }
}

/// Amenaza agregada del equipo rival que un objeto puede contrarrestar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatId {
    Physical,
    Magic,
    HardCc,
    Sustain,
    Tanks,
    Burst,
    Poke,
    Engage,
    Catch,
    Mobility,
    Dps,
    Splitpush,
    Siege,
    EarlyPressure,
    Scaling,
}

impl ThreatId {
    pub const ALL: [ThreatId; 15] = [
        ThreatId::Physical,
        ThreatId::Magic,
        ThreatId::HardCc,
        ThreatId::Sustain,
        ThreatId::Tanks,
        ThreatId::Burst,
        ThreatId::Poke,
        ThreatId::Engage,
        ThreatId::Catch,
        ThreatId::Mobility,
        ThreatId::Dps,
        ThreatId::Splitpush,
        ThreatId::Siege,
        ThreatId::EarlyPressure,
        ThreatId::Scaling,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Physical => "physical",
            Self::Magic => "magic",
            Self::HardCc => "hard_cc",
            Self::Sustain => "sustain",
            Self::Tanks => "tanks",
            Self::Burst => "burst",
            Self::Poke => "poke",
            Self::Engage => "engage",
            Self::Catch => "catch",
            Self::Mobility => "mobility",
            Self::Dps => "dps",
            Self::Splitpush => "splitpush",
            Self::Siege => "siege",
            Self::EarlyPressure => "early_pressure",
            Self::Scaling => "scaling",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Physical => "daño físico",
            Self::Magic => "daño mágico",
            Self::HardCc => "CC duro",
            Self::Sustain => "sustain o curación",
            Self::Tanks => "tanques o frontline",
            Self::Burst => "burst o assassins",
            Self::Poke => "poke",
            Self::Engage => "engage",
            Self::Catch => "catch",
            Self::Mobility => "movilidad",
            Self::Dps => "DPS sostenido",
            Self::Splitpush => "splitpush",
            Self::Siege => "asedio",
            Self::EarlyPressure => "presión temprana",
            Self::Scaling => "escalado tardío",
        }
    }

    /// Umbral de activación. Las señales por debajo no se usan para recomendar.
    pub fn threshold(self) -> f32 {
        match self {
            Self::Sustain | Self::Siege => 0.40,
            Self::HardCc => 0.42,
            Self::EarlyPressure => 0.50,
            Self::Scaling => 0.55,
            _ => 0.45,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitWeight {
    pub tag: FitTag,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreatWeight {
    pub threat: ThreatId,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemEntry {
    pub id: String,
    pub name: String,
    /// Identificador de Data Dragon para el icono local (`/items/<icon>.png`).
    pub icon: u32,
    pub category: ItemCategory,
    pub slot: ItemSlot,
    /// Peso curado de potencia general dentro de su clase, 0..1. No es una
    /// estadística de partidas: sirve para desempatar objetos de la misma función.
    pub baseline: f32,
    /// `true` si solo debe proponerse cuando el rival presenta alguna de las
    /// amenazas que el objeto contrarresta.
    #[serde(default)]
    pub situational_only: bool,
    /// Qué hace el objeto y contra qué se usa. Es la definición curada.
    pub function: String,
    pub fit: Vec<FitWeight>,
    #[serde(default)]
    pub counters: Vec<ThreatWeight>,
    #[serde(default)]
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunePageEntry {
    pub id: String,
    pub name: String,
    /// Identificador de la runa clave en Data Dragon (`/runes/<keystone_id>.png`).
    pub keystone_id: u32,
    pub primary_path: String,
    pub keystone: String,
    #[serde(default)]
    pub primary_runes: Vec<String>,
    pub secondary_path: String,
    #[serde(default)]
    pub secondary_runes: Vec<String>,
    #[serde(default)]
    pub shards: Vec<String>,
    /// Para qué sirve la página, en una frase.
    pub function: String,
    pub fit: Vec<FitWeight>,
    #[serde(default)]
    pub counters: Vec<ThreatWeight>,
    #[serde(default)]
    pub roles: Vec<Role>,
}

/// Plantilla curada de build para una familia de campeones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildTemplate {
    pub id: String,
    pub name: String,
    /// Objetos de inicio, en orden de preferencia.
    pub starter: Vec<String>,
    /// Botas posibles, en orden de preferencia.
    pub boots: Vec<String>,
    /// Núcleo de la build, en orden de compra.
    pub core: Vec<String>,
    /// Objetos situacionales: solo se muestran si encajan o si el rival
    /// presenta la amenaza que contrarrestan.
    pub situational: Vec<String>,
    #[serde(default)]
    pub runes: Vec<String>,
}

/// Asignación curada de un campeón concreto a una plantilla.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChampionBuild {
    pub champion: u16,
    pub template: String,
    /// Sustituye por completo el núcleo de la plantilla cuando no está vacío.
    #[serde(default)]
    pub core: Vec<String>,
    /// Sustituye por completo las páginas de runas de la plantilla cuando no
    /// está vacío.
    #[serde(default)]
    pub runes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemCatalog {
    pub schema: String,
    pub version: String,
    pub provenance: Provenance,
    pub items: Vec<ItemEntry>,
    pub rune_pages: Vec<RunePageEntry>,
    #[serde(default)]
    pub templates: Vec<BuildTemplate>,
    #[serde(default)]
    pub champion_builds: Vec<ChampionBuild>,
}

impl ItemCatalog {
    /// Parsea y valida el catálogo incluido en el binario.
    pub fn seed() -> Result<Self, ItemError> {
        let catalog: Self = serde_json::from_str(include_str!("../../data/items-v1.json"))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), ItemError> {
        if self.schema != ITEM_CATALOG_SCHEMA {
            return Err(ItemError::UnsupportedSchema {
                found: self.schema.clone(),
                expected: ITEM_CATALOG_SCHEMA,
            });
        }
        require_text("catálogo", "version", &self.version)?;
        if !self.provenance.is_complete() {
            return Err(ItemError::IncompleteProvenance);
        }
        if self.items.is_empty() {
            return Err(ItemError::NoItems);
        }
        if self.rune_pages.is_empty() {
            return Err(ItemError::NoRunePages);
        }

        let mut item_ids = HashSet::with_capacity(self.items.len());
        for item in &self.items {
            require_text("objeto", &item.id, &item.name)?;
            require_text("objeto", &item.id, &item.function)?;
            if !item_ids.insert(item.id.as_str()) {
                return Err(ItemError::DuplicateId {
                    section: "objeto",
                    id: item.id.clone(),
                });
            }
            if item.icon == 0 {
                return Err(ItemError::MissingIcon {
                    section: "objeto",
                    id: item.id.clone(),
                });
            }
            validate_unit_weight("objeto", &item.id, "baseline", item.baseline)?;
            if item.situational_only && item.counters.is_empty() {
                return Err(ItemError::InvalidParams(
                    "un objeto situacional debe declarar al menos una amenaza que contrarreste",
                ));
            }
            validate_fit("objeto", &item.id, &item.fit)?;
            validate_counters("objeto", &item.id, &item.counters)?;
            validate_roles("objeto", &item.id, &item.roles)?;
        }

        let mut rune_ids = HashSet::with_capacity(self.rune_pages.len());
        for page in &self.rune_pages {
            require_text("runas", &page.id, &page.name)?;
            require_text("runas", &page.id, &page.function)?;
            require_text("runas", &page.id, &page.primary_path)?;
            require_text("runas", &page.id, &page.keystone)?;
            require_text("runas", &page.id, &page.secondary_path)?;
            if !rune_ids.insert(page.id.as_str()) {
                return Err(ItemError::DuplicateId {
                    section: "runas",
                    id: page.id.clone(),
                });
            }
            if page.keystone_id == 0 {
                return Err(ItemError::MissingIcon {
                    section: "runas",
                    id: page.id.clone(),
                });
            }
            validate_fit("runas", &page.id, &page.fit)?;
            validate_counters("runas", &page.id, &page.counters)?;
            validate_roles("runas", &page.id, &page.roles)?;
            for rune in page
                .primary_runes
                .iter()
                .chain(page.secondary_runes.iter())
                .chain(page.shards.iter())
            {
                require_text("runas", &page.id, rune)?;
            }
        }

        let mut template_ids = HashSet::with_capacity(self.templates.len());
        for template in &self.templates {
            require_text("plantilla", &template.id, &template.name)?;
            if !template_ids.insert(template.id.as_str()) {
                return Err(ItemError::DuplicateId {
                    section: "plantilla",
                    id: template.id.clone(),
                });
            }
            if template.core.is_empty() {
                return Err(ItemError::EmptyText {
                    section: "plantilla",
                    id: template.id.clone(),
                    field: "core",
                });
            }
            if template.runes.is_empty() {
                return Err(ItemError::EmptyText {
                    section: "plantilla",
                    id: template.id.clone(),
                    field: "runes",
                });
            }
            for id in template
                .starter
                .iter()
                .chain(template.boots.iter())
                .chain(template.core.iter())
                .chain(template.situational.iter())
            {
                if !item_ids.contains(id.as_str()) {
                    return Err(ItemError::UnknownReference {
                        section: "plantilla",
                        id: template.id.clone(),
                        reference: id.clone(),
                    });
                }
            }
            for id in &template.runes {
                if !rune_ids.contains(id.as_str()) {
                    return Err(ItemError::UnknownReference {
                        section: "plantilla",
                        id: template.id.clone(),
                        reference: id.clone(),
                    });
                }
            }
        }

        let mut champions = HashSet::with_capacity(self.champion_builds.len());
        for build in &self.champion_builds {
            if build.champion == 0 {
                return Err(ItemError::EmptyText {
                    section: "campeón",
                    id: "0".to_string(),
                    field: "champion",
                });
            }
            if !champions.insert(build.champion) {
                return Err(ItemError::DuplicateId {
                    section: "campeón",
                    id: build.champion.to_string(),
                });
            }
            if !template_ids.contains(build.template.as_str()) {
                return Err(ItemError::UnknownReference {
                    section: "campeón",
                    id: build.champion.to_string(),
                    reference: build.template.clone(),
                });
            }
            for id in &build.core {
                if !item_ids.contains(id.as_str()) {
                    return Err(ItemError::UnknownReference {
                        section: "campeón",
                        id: build.champion.to_string(),
                        reference: id.clone(),
                    });
                }
            }
            for id in &build.runes {
                if !rune_ids.contains(id.as_str()) {
                    return Err(ItemError::UnknownReference {
                        section: "campeón",
                        id: build.champion.to_string(),
                        reference: id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn item(&self, id: &str) -> Option<&ItemEntry> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn rune_page(&self, id: &str) -> Option<&RunePageEntry> {
        self.rune_pages.iter().find(|page| page.id == id)
    }

    pub fn template(&self, id: &str) -> Option<&BuildTemplate> {
        self.templates.iter().find(|template| template.id == id)
    }

    pub fn champion_build(&self, champion: ChampionKey) -> Option<&ChampionBuild> {
        self.champion_builds
            .iter()
            .find(|build| build.champion == champion.0)
    }
}

/// Catálogo incluido en el binario. Un fallo aquí es un error de build, no una
/// condición de ejecución.
pub fn item_catalog() -> &'static ItemCatalog {
    static CATALOG: OnceLock<ItemCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        ItemCatalog::seed().expect("el catálogo de objetos incluido debe ser válido")
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ItemError {
    #[error("no se pudo leer el catálogo de objetos: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("esquema de catálogo no soportado: {found}; se esperaba {expected}")]
    UnsupportedSchema {
        found: String,
        expected: &'static str,
    },
    #[error("la procedencia del catálogo está incompleta")]
    IncompleteProvenance,
    #[error("el catálogo no tiene objetos")]
    NoItems,
    #[error("el catálogo no tiene páginas de runas")]
    NoRunePages,
    #[error("{section} `{id}` tiene el campo `{field}` vacío")]
    EmptyText {
        section: &'static str,
        id: String,
        field: &'static str,
    },
    #[error("{section} duplicado: `{id}`")]
    DuplicateId { section: &'static str, id: String },
    #[error("{section} `{id}` no declara icono")]
    MissingIcon { section: &'static str, id: String },
    #[error("{section} `{id}` referencia un id desconocido: {reference}")]
    UnknownReference {
        section: &'static str,
        id: String,
        reference: String,
    },
    #[error("{section} `{id}` no declara afinidad funcional")]
    EmptyFit { section: &'static str, id: String },
    #[error("{section} `{id}` tiene un peso inválido en `{field}`: {value}")]
    InvalidWeight {
        section: &'static str,
        id: String,
        field: &'static str,
        value: f32,
    },
    #[error("{section} `{id}` repite el rol `{role}`")]
    DuplicateRole {
        section: &'static str,
        id: String,
        role: &'static str,
    },
    #[error("parámetros de loadout inválidos: {0}")]
    InvalidParams(&'static str),
}

fn require_text(section: &'static str, id: &str, value: &str) -> Result<(), ItemError> {
    if value.trim().is_empty() {
        return Err(ItemError::EmptyText {
            section,
            id: id.to_string(),
            field: "texto",
        });
    }
    Ok(())
}

fn validate_unit_weight(
    section: &'static str,
    id: &str,
    field: &'static str,
    value: f32,
) -> Result<(), ItemError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(ItemError::InvalidWeight {
            section,
            id: id.to_string(),
            field,
            value,
        });
    }
    Ok(())
}

fn validate_fit(section: &'static str, id: &str, fit: &[FitWeight]) -> Result<(), ItemError> {
    if fit.is_empty() {
        return Err(ItemError::EmptyFit {
            section,
            id: id.to_string(),
        });
    }
    let mut seen = BTreeSet::new();
    for entry in fit {
        if !seen.insert(entry.tag) {
            return Err(ItemError::DuplicateId {
                section,
                id: format!("{id}:fit:{}", entry.tag.as_str()),
            });
        }
        validate_unit_weight(section, id, "fit", entry.weight)?;
    }
    Ok(())
}

fn validate_counters(
    section: &'static str,
    id: &str,
    counters: &[ThreatWeight],
) -> Result<(), ItemError> {
    let mut seen = BTreeSet::new();
    for entry in counters {
        if !seen.insert(entry.threat) {
            return Err(ItemError::DuplicateId {
                section,
                id: format!("{id}:counter:{}", entry.threat.as_str()),
            });
        }
        validate_unit_weight(section, id, "counter", entry.weight)?;
    }
    Ok(())
}

fn validate_roles(section: &'static str, id: &str, roles: &[Role]) -> Result<(), ItemError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(*role) {
            return Err(ItemError::DuplicateRole {
                section,
                id: id.to_string(),
                role: role.as_str(),
            });
        }
    }
    Ok(())
}

/// Señal de amenaza ya resumida para la UI y para puntuar objetos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatSignal {
    pub id: ThreatId,
    pub label_es: String,
    pub value_low: f32,
    pub value_high: f32,
    pub confidence: f32,
    /// Nivel ponderado por confianza, 0..1.
    pub severity: f32,
    pub confirmed: bool,
}

/// Detecta las amenazas del rival que tienen datos suficientes para usarse.
///
/// Una cobertura sin celdas conocidas no activa nada: el intervalo superior de
/// un equipo sin perfiles no se interpreta como amenaza real.
pub fn detect_threats(enemy: &TeamAnalysis) -> Vec<ThreatSignal> {
    let mut signals = Vec::new();

    if let Some(damage) = &enemy.damage {
        if damage.known_weight_ratio >= 0.45 && damage.confidence >= 0.10 {
            push_damage_signal(
                &mut signals,
                ThreatId::Physical,
                damage.physical,
                damage.confidence,
            );
            push_damage_signal(
                &mut signals,
                ThreatId::Magic,
                damage.magic,
                damage.confidence,
            );
        }
    }

    push_coverage_signal(
        &mut signals,
        ThreatId::Sustain,
        enemy.archetypes.get(&ArchetypeId::Sustain),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Tanks,
        best_archetype(enemy, &[ArchetypeId::Frontline, ArchetypeId::Tank]),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Burst,
        best_archetype(enemy, &[ArchetypeId::Burst, ArchetypeId::Assassin]),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Poke,
        enemy.archetypes.get(&ArchetypeId::Poke),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Engage,
        enemy.archetypes.get(&ArchetypeId::Engage),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Catch,
        enemy.archetypes.get(&ArchetypeId::Catch),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Dps,
        enemy.archetypes.get(&ArchetypeId::Dps),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Splitpush,
        enemy.archetypes.get(&ArchetypeId::Splitpush),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Siege,
        enemy.archetypes.get(&ArchetypeId::Siege),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::EarlyPressure,
        enemy.archetypes.get(&ArchetypeId::EarlyGame),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Scaling,
        enemy.archetypes.get(&ArchetypeId::Scaling),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::HardCc,
        enemy.traits.get(&TraitId::HardCc),
    );
    push_coverage_signal(
        &mut signals,
        ThreatId::Mobility,
        enemy.traits.get(&TraitId::Mobility),
    );

    signals.sort_by(|left, right| {
        right
            .severity
            .total_cmp(&left.severity)
            .then_with(|| left.id.cmp(&right.id))
    });
    signals
}

fn best_archetype<'a>(team: &'a TeamAnalysis, ids: &[ArchetypeId]) -> Option<&'a Coverage> {
    ids.iter()
        .filter_map(|id| team.archetypes.get(id))
        .max_by(|left, right| left.midpoint().total_cmp(&right.midpoint()))
}

fn push_coverage_signal(out: &mut Vec<ThreatSignal>, id: ThreatId, coverage: Option<&Coverage>) {
    let Some(coverage) = coverage else {
        return;
    };
    if coverage.total == 0 || coverage.known == 0 || coverage.confidence < 0.10 {
        return;
    }
    let threshold = id.threshold();
    if coverage.value_high < threshold {
        return;
    }
    let severity =
        clamp_unit(coverage.midpoint() * (0.45 + 0.55 * clamp_unit(coverage.confidence)));
    if severity < 0.15 {
        return;
    }
    out.push(ThreatSignal {
        id,
        label_es: id.label_es().to_string(),
        value_low: coverage.value_low,
        value_high: coverage.value_high,
        confidence: coverage.confidence,
        severity,
        confirmed: coverage.value_low >= threshold,
    });
}

fn push_damage_signal(out: &mut Vec<ThreatSignal>, id: ThreatId, share: f32, confidence: f32) {
    let share = clamp_unit(share);
    if share < id.threshold() {
        return;
    }
    let severity = clamp_unit(share * (0.45 + 0.55 * clamp_unit(confidence)));
    if severity < 0.15 {
        return;
    }
    out.push(ThreatSignal {
        id,
        label_es: id.label_es().to_string(),
        value_low: share,
        value_high: share,
        confidence,
        severity,
        confirmed: share >= 0.55,
    });
}

/// Parámetros de la recomendación de objetos y runas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadoutParams {
    pub version: u32,
    pub min_score: f32,
    pub core_count: usize,
    pub situational_count: usize,
    pub rune_count: usize,
}

impl Default for LoadoutParams {
    fn default() -> Self {
        Self {
            version: 1,
            min_score: 0.18,
            core_count: 3,
            situational_count: 4,
            rune_count: 3,
        }
    }
}

impl LoadoutParams {
    pub fn validate(&self) -> Result<(), ItemError> {
        if self.version == 0 {
            return Err(ItemError::InvalidParams(
                "la versión debe ser mayor que cero",
            ));
        }
        if !self.min_score.is_finite() || !(0.0..=1.0).contains(&self.min_score) {
            return Err(ItemError::InvalidParams("min_score debe estar entre 0 y 1"));
        }
        if self.core_count == 0 || self.core_count > 10 {
            return Err(ItemError::InvalidParams(
                "core_count debe estar entre 1 y 10",
            ));
        }
        if self.situational_count > 10 {
            return Err(ItemError::InvalidParams(
                "situational_count no puede superar 10",
            ));
        }
        if self.rune_count == 0 || self.rune_count > 10 {
            return Err(ItemError::InvalidParams(
                "rune_count debe estar entre 1 y 10",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSource {
    /// Perfil estratégico del conjunto activo.
    Profile,
    /// Solo tags de Riot porque el campeón no tiene perfil.
    RiotTags,
    /// Ni perfil ni tags: no hay encaje que calcular.
    None,
}

impl ProfileSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::RiotTags => "riot_tags",
            Self::None => "none",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Profile => "perfil estratégico",
            Self::RiotTags => "tags Riot",
            Self::None => "sin datos",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadoutReason {
    pub code: String,
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSuggestion {
    pub id: String,
    pub name: String,
    /// Identificador de Data Dragon para el icono local.
    pub icon: u32,
    pub category: ItemCategory,
    pub function: String,
    pub score: f32,
    pub reasons: Vec<LoadoutReason>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuneSuggestion {
    pub id: String,
    pub name: String,
    pub keystone_id: u32,
    pub primary_path: String,
    pub keystone: String,
    pub primary_runes: Vec<String>,
    pub secondary_path: String,
    pub secondary_runes: Vec<String>,
    pub shards: Vec<String>,
    pub function: String,
    pub score: f32,
    pub reasons: Vec<LoadoutReason>,
}

/// De dónde sale la base de la build: una plantilla curada o las reglas
/// genéricas por función.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSource {
    Template,
    Rules,
}

impl BuildSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Template => "template",
            Self::Rules => "rules",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Template => "plantilla curada",
            Self::Rules => "reglas por función",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadoutRecommendation {
    pub champion: ChampionKey,
    pub champion_name: String,
    pub role: Option<Role>,
    pub starter: Option<ItemSuggestion>,
    pub boots: Option<ItemSuggestion>,
    pub core: Vec<ItemSuggestion>,
    pub situational: Vec<ItemSuggestion>,
    pub runes: Vec<RuneSuggestion>,
    pub threats: Vec<ThreatSignal>,
    pub profile_source: ProfileSource,
    /// `true` si todas las celdas conocidas del perfil propio son inferidas.
    pub profile_inferred: bool,
    pub caveats: Vec<String>,
    pub catalog_version: String,
    pub rules_version: String,
    pub enemy_picks: usize,
    pub enemy_confidence: f32,
    /// Nombre de la plantilla curada aplicada, si existe.
    pub build_name: Option<String>,
    pub build_source: BuildSource,
}

pub struct LoadoutRequest<'a> {
    pub champion: ChampionKey,
    pub champion_name: &'a str,
    pub role: Option<Role>,
    pub riot_tags: &'a [String],
    pub profile: Option<&'a ChampionProfile>,
    pub enemy: Option<&'a TeamAnalysis>,
    pub enemy_picks: usize,
    pub catalog: &'a ItemCatalog,
    pub params: &'a LoadoutParams,
}

/// Calcula objetos y runas para un campeón frente al equipo rival.
///
/// Si el campeón tiene una plantilla curada se respeta su orden de compra y se
/// ajustan botas, situacionales y runas al rival. Sin plantilla se usan las
/// reglas por función, que exigen coherencia de daño y no proponen objetos
/// situacionales sin la amenaza correspondiente.
pub fn recommend_loadout(request: &LoadoutRequest<'_>) -> Result<LoadoutRecommendation, ItemError> {
    request.params.validate()?;
    let affinity = build_affinity(request.profile, request.riot_tags);
    let profile_source = if affinity.profile_cells > 0 {
        ProfileSource::Profile
    } else if !request.riot_tags.is_empty() {
        ProfileSource::RiotTags
    } else {
        ProfileSource::None
    };
    let profile_inferred = request.profile.is_some_and(profile_is_inferred);
    let threats = request.enemy.map(detect_threats).unwrap_or_default();
    let role = request
        .role
        .or_else(|| request.profile.and_then(primary_role));

    let champion_build = request.catalog.champion_build(request.champion);
    let template = champion_build.and_then(|build| request.catalog.template(&build.template));
    let build_source = if template.is_some() {
        BuildSource::Template
    } else {
        BuildSource::Rules
    };
    let build_name = template.map(|template| template.name.clone());

    let starter;
    let boots;
    let core;
    let situational;
    let runes;

    if let Some(template) = template {
        starter = best_ordered_item(
            &template.starter,
            request,
            &affinity,
            &threats,
            profile_source,
            role,
        );
        boots = best_ordered_item(
            &template.boots,
            request,
            &affinity,
            &threats,
            profile_source,
            role,
        );

        let core_ids = champion_build
            .map(|build| build.core.as_slice())
            .filter(|ids| !ids.is_empty())
            .unwrap_or(template.core.as_slice());
        let mut built = items_for_ids(
            core_ids,
            request,
            &affinity,
            &threats,
            profile_source,
            role,
            true,
        );
        // Un campeón jugado fuera del rol de su plantilla (p. ej. un tirador de
        // support) no debe quedarse sin núcleo. Si el filtro de rol lo vacía,
        // se aplica igualmente la plantilla curada del campeón.
        if built.is_empty() && role.is_some() {
            built = items_for_ids(
                core_ids,
                request,
                &affinity,
                &threats,
                profile_source,
                None,
                true,
            );
        }
        let core_ids: HashSet<String> = built.iter().map(|item| item.id.clone()).collect();
        core = built;

        let mut pool = items_for_ids(
            &template.situational,
            request,
            &affinity,
            &threats,
            profile_source,
            role,
            false,
        );
        pool.retain(|item| !core_ids.contains(&item.id));
        sort_items(&mut pool);
        pool.dedup_by(|left, right| left.id == right.id);
        situational = pool
            .into_iter()
            .take(request.params.situational_count)
            .collect();

        let rune_ids = champion_build
            .map(|build| build.runes.as_slice())
            .filter(|ids| !ids.is_empty())
            .unwrap_or(template.runes.as_slice());
        runes = collect_runes(
            rune_ids.iter().map(String::as_str),
            request,
            &affinity,
            &threats,
            profile_source,
            role,
            0.0,
        );
    } else {
        let mut starters = Vec::new();
        let mut boots_pool = Vec::new();
        let mut identity: Vec<ItemSuggestion> = Vec::new();
        let mut extras: Vec<ItemSuggestion> = Vec::new();
        for entry in &request.catalog.items {
            if !role_allows(&entry.roles, role) || !damage_compatible(entry, &affinity) {
                continue;
            }
            let Some(suggestion) = item_suggestion(
                entry,
                &affinity,
                &threats,
                profile_source,
                request.params.min_score,
                false,
            ) else {
                continue;
            };
            match entry.slot {
                ItemSlot::Starter => starters.push(suggestion),
                ItemSlot::Boots => boots_pool.push(suggestion),
                ItemSlot::Item => {
                    if item_fit(entry, &affinity) >= 0.15 {
                        identity.push(suggestion);
                    } else {
                        extras.push(suggestion);
                    }
                }
            }
        }
        sort_items(&mut starters);
        sort_items(&mut boots_pool);
        sort_items(&mut identity);
        sort_items(&mut extras);
        starter = starters.into_iter().next();
        boots = boots_pool.into_iter().next();

        let mut rest = identity.split_off(identity.len().min(request.params.core_count));
        core = identity;
        rest.extend(extras);
        sort_items(&mut rest);
        situational = rest
            .into_iter()
            .take(request.params.situational_count)
            .collect();

        runes = collect_runes(
            request
                .catalog
                .rune_pages
                .iter()
                .map(|page| page.id.as_str()),
            request,
            &affinity,
            &threats,
            profile_source,
            role,
            request.params.min_score,
        );
    }

    let mut caveats = Vec::new();
    if profile_source == ProfileSource::RiotTags {
        caveats.push(format!(
            "No hay perfil estratégico de {}; el encaje se apoya en sus tags Riot ({}). No es una lectura curada del kit.",
            request.champion_name,
            request
                .riot_tags
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else if profile_source == ProfileSource::None {
        caveats.push(format!(
            "No hay perfil ni tags de {}; no se puede calcular un encaje fiable.",
            request.champion_name
        ));
    }
    if profile_inferred {
        caveats.push(format!(
            "El perfil estratégico de {} es inferido (piloto), no curado: el encaje hereda esa incertidumbre.",
            request.champion_name
        ));
    }
    match build_source {
        BuildSource::Template => caveats.push(format!(
            "La base (inicio, botas y núcleo) sigue la plantilla curada «{}». No es una build observada ni una probabilidad de victoria: el orden es una recomendación de compra.",
            build_name.as_deref().unwrap_or("sin nombre")
        )),
        BuildSource::Rules => caveats.push(
            "No hay plantilla curada para este campeón: la build se deriva solo de reglas por función y de las amenazas del rival."
                .to_string(),
        ),
    }
    caveats.push(
        "Los objetos situacionales solo aparecen si encajan con el campeón o si el rival presenta la amenaza que contrarrestan."
            .to_string(),
    );

    let enemy_picks = request.enemy_picks;
    if enemy_picks == 0 {
        caveats.push(
            "No hay rivales leídos: las sugerencias se basan solo en el encaje del campeón."
                .to_string(),
        );
    } else if request.enemy.map(|enemy| enemy.partial).unwrap_or(true) {
        caveats.push(format!(
            "Lectura parcial del rival ({enemy_picks} picks): las amenazas sin perfil no cuentan como confirmadas."
        ));
    }
    if !threats.is_empty() {
        caveats.push(
            "Las amenazas se derivan de perfiles estratégicos y del reparto de daño; no son estadísticas de partidas."
                .to_string(),
        );
    }
    if core.is_empty() && runes.is_empty() {
        caveats.push(
            "No hay objetos ni runas con encaje suficiente para este campeón con los datos actuales."
                .to_string(),
        );
    }

    Ok(LoadoutRecommendation {
        champion: request.champion,
        champion_name: request.champion_name.to_string(),
        role,
        starter,
        boots,
        core,
        situational,
        runes,
        threats,
        profile_source,
        profile_inferred,
        caveats,
        catalog_version: request.catalog.version.clone(),
        rules_version: LOADOUT_RULES_VERSION.to_string(),
        enemy_picks,
        enemy_confidence: request.enemy.map(|enemy| enemy.confidence).unwrap_or(0.0),
        build_name,
        build_source,
    })
}

/// Recoge los objetos de una lista curada que respetan rol y coherencia de daño.
fn items_for_ids(
    ids: &[String],
    request: &LoadoutRequest<'_>,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    role: Option<Role>,
    curated_pick: bool,
) -> Vec<ItemSuggestion> {
    let mut seen = HashSet::new();
    ids.iter()
        .filter_map(|id| {
            let entry = request.catalog.item(id)?;
            if !role_allows(&entry.roles, role) || !damage_compatible(entry, affinity) {
                return None;
            }
            item_suggestion(entry, affinity, threats, profile_source, 0.0, curated_pick)
        })
        .filter(|item| seen.insert(item.id.clone()))
        .collect()
}

/// Elige el mejor objeto de una lista ordenada de la plantilla respetando el
/// rol y la coherencia de daño. El orden de la plantilla desempata.
fn best_ordered_item(
    ids: &[String],
    request: &LoadoutRequest<'_>,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    role: Option<Role>,
) -> Option<ItemSuggestion> {
    let best = best_ordered_item_for_role(ids, request, affinity, threats, profile_source, role);
    if best.is_some() || role.is_none() {
        return best;
    }
    // Fuera del rol de la plantilla (p. ej. un support jugando un campeón de
    // mid) se conserva el objeto curado antes que dejar el hueco vacío.
    best_ordered_item_for_role(ids, request, affinity, threats, profile_source, None)
}

fn best_ordered_item_for_role(
    ids: &[String],
    request: &LoadoutRequest<'_>,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    role: Option<Role>,
) -> Option<ItemSuggestion> {
    let mut best: Option<(f32, ItemSuggestion)> = None;
    let mut order = 0.0_f32;
    for id in ids {
        order += 1.0;
        let Some(entry) = request.catalog.item(id) else {
            continue;
        };
        if !role_allows(&entry.roles, role) || !damage_compatible(entry, affinity) {
            continue;
        }
        let Some(suggestion) = item_suggestion(entry, affinity, threats, profile_source, 0.0, true)
        else {
            continue;
        };
        // El orden curado de la plantilla desempata: no se adelanta un objeto
        // defensivo solo porque el rival tenga una amenaza.
        let rank = suggestion.score + 0.02 * (ids.len() as f32 - order) / ids.len() as f32;
        if best.as_ref().is_none_or(|(score, _)| rank > *score) {
            best = Some((rank, suggestion));
        }
    }
    best.map(|(_, suggestion)| suggestion)
}

fn collect_runes<'a, I>(
    ids: I,
    request: &LoadoutRequest<'_>,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    role: Option<Role>,
    min_score: f32,
) -> Vec<RuneSuggestion>
where
    I: IntoIterator<Item = &'a str>,
{
    let ids: Vec<&str> = ids.into_iter().collect();
    let mut runes = collect_runes_for_role(
        ids.iter().copied(),
        request,
        affinity,
        threats,
        profile_source,
        role,
        min_score,
    );
    if runes.is_empty() && role.is_some() {
        // Igual que con los objetos: fuera del rol de la plantilla se conserva
        // la página curada del campeón antes que no proponer runas.
        runes = collect_runes_for_role(
            ids.iter().copied(),
            request,
            affinity,
            threats,
            profile_source,
            None,
            min_score,
        );
    }
    runes
}

fn collect_runes_for_role<'a, I>(
    ids: I,
    request: &LoadoutRequest<'_>,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    role: Option<Role>,
    min_score: f32,
) -> Vec<RuneSuggestion>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut runes: Vec<RuneSuggestion> = ids
        .into_iter()
        .filter_map(|id| {
            let page = request.catalog.rune_page(id)?;
            if !role_allows(&page.roles, role) {
                return None;
            }
            rune_suggestion(page, affinity, threats, profile_source, min_score)
        })
        .collect();
    runes.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.id.cmp(&right.id))
    });
    runes.dedup_by(|left, right| left.id == right.id);
    runes.truncate(request.params.rune_count);
    runes
}

fn profile_is_inferred(profile: &ChampionProfile) -> bool {
    let mut known = false;
    for rating in profile.archetypes.values() {
        if rating.intensity.is_none() {
            continue;
        }
        known = true;
        if rating.source.is_observed() {
            return false;
        }
    }
    for rating in profile.traits.values() {
        if rating.value.is_none() {
            continue;
        }
        known = true;
        if rating.source.is_observed() {
            return false;
        }
    }
    known
}

fn primary_role(profile: &ChampionProfile) -> Option<Role> {
    profile
        .roles
        .iter()
        .max_by(|left, right| left.proficiency.total_cmp(&right.proficiency))
        .map(|role| role.role)
}

fn role_allows(roles: &[Role], role: Option<Role>) -> bool {
    match role {
        Some(role) => roles.is_empty() || roles.contains(&role),
        None => true,
    }
}

fn sort_items(items: &mut [ItemSuggestion]) {
    items.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.name.cmp(&right.name))
    });
}

struct Affinity {
    values: BTreeMap<FitTag, f32>,
    profile_cells: usize,
}

impl Affinity {
    fn value(&self, tag: FitTag) -> f32 {
        self.values.get(&tag).copied().unwrap_or(0.0)
    }
}

fn build_affinity(profile: Option<&ChampionProfile>, riot_tags: &[String]) -> Affinity {
    let mut sums: BTreeMap<FitTag, (f32, f32)> = BTreeMap::new();
    let mut profile_cells = 0;

    if let Some(profile) = profile {
        for (id, rating) in &profile.archetypes {
            let Some(value) = rating.intensity else {
                continue;
            };
            profile_cells += 1;
            for (tag, weight) in archetype_fit(*id) {
                add_fit(&mut sums, *tag, value, *weight);
            }
        }
        for (id, rating) in &profile.traits {
            let Some(value) = rating.value else {
                continue;
            };
            profile_cells += 1;
            for (tag, weight) in trait_fit(*id) {
                add_fit(&mut sums, *tag, value, *weight * 0.60);
            }
        }
    }

    let riot_weight = if profile.is_some() { 0.30 } else { 0.65 };
    for tag in riot_tags {
        for (fit, weight) in riot_fit(tag) {
            add_fit(&mut sums, *fit, 1.0, *weight * riot_weight);
        }
    }

    let values = sums
        .into_iter()
        .filter_map(|(tag, (sum, weight))| {
            (weight > 0.0).then_some((tag, clamp_unit(sum / weight)))
        })
        .collect();

    Affinity {
        values,
        profile_cells,
    }
}

fn add_fit(sums: &mut BTreeMap<FitTag, (f32, f32)>, tag: FitTag, value: f32, weight: f32) {
    if value <= 0.0 || weight <= 0.0 {
        return;
    }
    let entry = sums.entry(tag).or_insert((0.0, 0.0));
    entry.0 += value * weight;
    entry.1 += weight;
}

fn item_fit(entry: &ItemEntry, affinity: &Affinity) -> f32 {
    weighted_affinity(&entry.fit, affinity)
}

fn rune_fit(page: &RunePageEntry, affinity: &Affinity) -> f32 {
    weighted_affinity(&page.fit, affinity)
}

fn weighted_affinity(fit: &[FitWeight], affinity: &Affinity) -> f32 {
    let total: f32 = fit.iter().map(|entry| entry.weight).sum();
    if total <= 0.0 {
        return 0.0;
    }
    fit.iter()
        .map(|entry| entry.weight * affinity.value(entry.tag))
        .sum::<f32>()
        / total
}

fn counter_score<'a>(
    counters: &[ThreatWeight],
    threats: &'a [ThreatSignal],
) -> (f32, Vec<(&'a ThreatSignal, f32)>) {
    let mut score = 0.0;
    let mut matched = Vec::new();
    for counter in counters {
        let Some(signal) = threats.iter().find(|signal| signal.id == counter.threat) else {
            continue;
        };
        score += counter.weight * signal.severity;
        matched.push((signal, counter.weight));
    }
    matched
        .sort_by(|left, right| (right.1 * right.0.severity).total_cmp(&(left.1 * left.0.severity)));
    (clamp_unit(score), matched)
}

fn item_suggestion(
    entry: &ItemEntry,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    min_score: f32,
    curated_pick: bool,
) -> Option<ItemSuggestion> {
    let fit = item_fit(entry, affinity);
    let (threat_score, matched) = counter_score(&entry.counters, threats);
    if !curated_pick && fit <= 0.01 && threat_score <= 0.01 {
        return None;
    }
    // Un objeto marcado como situacional no aparece si el rival no presenta la
    // amenaza que contrarresta. Esto evita el anti-curación forzado.
    if !curated_pick && entry.situational_only && matched.is_empty() {
        return None;
    }
    let score = clamp_unit(0.10 + 0.50 * fit + 0.35 * threat_score + 0.05 * entry.baseline);
    if score < min_score {
        return None;
    }

    let mut reasons = vec![LoadoutReason {
        code: "function".to_string(),
        label: "Función".to_string(),
        detail: entry.function.clone(),
    }];
    push_counter_reasons(&mut reasons, &matched);
    push_fit_reasons(&mut reasons, &entry.fit, affinity, profile_source);

    Some(ItemSuggestion {
        id: entry.id.clone(),
        name: entry.name.clone(),
        icon: entry.icon,
        category: entry.category,
        function: entry.function.clone(),
        score,
        reasons,
    })
}

fn rune_suggestion(
    page: &RunePageEntry,
    affinity: &Affinity,
    threats: &[ThreatSignal],
    profile_source: ProfileSource,
    min_score: f32,
) -> Option<RuneSuggestion> {
    let fit = rune_fit(page, affinity);
    let (threat_score, matched) = counter_score(&page.counters, threats);
    if fit <= 0.01 && threat_score <= 0.01 {
        return None;
    }
    let score = clamp_unit(0.12 + 0.58 * fit + 0.30 * threat_score);
    if score < min_score {
        return None;
    }

    let mut reasons = vec![LoadoutReason {
        code: "function".to_string(),
        label: "Función".to_string(),
        detail: page.function.clone(),
    }];
    push_counter_reasons(&mut reasons, &matched);
    push_fit_reasons(&mut reasons, &page.fit, affinity, profile_source);

    Some(RuneSuggestion {
        id: page.id.clone(),
        name: page.name.clone(),
        keystone_id: page.keystone_id,
        primary_path: page.primary_path.clone(),
        keystone: page.keystone.clone(),
        primary_runes: page.primary_runes.clone(),
        secondary_path: page.secondary_path.clone(),
        secondary_runes: page.secondary_runes.clone(),
        shards: page.shards.clone(),
        function: page.function.clone(),
        score,
        reasons,
    })
}

fn fit_weight(fit: &[FitWeight], tag: FitTag) -> f32 {
    fit.iter()
        .find(|entry| entry.tag == tag)
        .map_or(0.0, |entry| entry.weight)
}

/// Evita mezclar objetos de daño físico y mágico en campeones claramente
/// inclinados hacia un tipo de daño. Un campeón híbrido acepta ambos.
fn damage_compatible(entry: &ItemEntry, affinity: &Affinity) -> bool {
    let item_ad = fit_weight(&entry.fit, FitTag::Ad);
    let item_ap = fit_weight(&entry.fit, FitTag::Ap);
    let champion_ad = affinity.value(FitTag::Ad);
    let champion_ap = affinity.value(FitTag::Ap);
    if item_ad >= 0.5 && item_ap < 0.2 && champion_ap > champion_ad + 0.25 {
        return false;
    }
    if item_ap >= 0.5 && item_ad < 0.2 && champion_ad > champion_ap + 0.25 {
        return false;
    }
    true
}

fn push_counter_reasons(reasons: &mut Vec<LoadoutReason>, matched: &[(&ThreatSignal, f32)]) {
    for (signal, _) in matched.iter().take(3) {
        reasons.push(LoadoutReason {
            code: format!("threat:{}", signal.id.as_str()),
            label: format!("Contra {}", signal.label_es),
            detail: format!(
                "El rival presenta {} (nivel {:.0} %, {}).",
                signal.label_es,
                signal.severity * 100.0,
                if signal.confirmed {
                    "confirmado"
                } else {
                    "posible"
                }
            ),
        });
    }
}

fn push_fit_reasons(
    reasons: &mut Vec<LoadoutReason>,
    fit: &[FitWeight],
    affinity: &Affinity,
    profile_source: ProfileSource,
) {
    let mut ranked: Vec<(FitTag, f32)> = fit
        .iter()
        .map(|entry| (entry.tag, entry.weight * affinity.value(entry.tag)))
        .filter(|(_, value)| *value > 0.01)
        .collect();
    ranked.sort_by(|left, right| right.1.total_cmp(&left.1));
    for (tag, _) in ranked.iter().take(2) {
        let value = affinity.value(*tag);
        reasons.push(LoadoutReason {
            code: format!("fit:{}", tag.as_str()),
            label: "Encaje con tu campeón".to_string(),
            detail: format!(
                "Afinidad de {} ({:.0} %) según {}.",
                tag.label_es(),
                value * 100.0,
                profile_source.label_es()
            ),
        });
    }
}

fn archetype_fit(id: ArchetypeId) -> &'static [(FitTag, f32)] {
    match id {
        ArchetypeId::Engage => &[
            (FitTag::Engage, 1.0),
            (FitTag::Frontline, 0.5),
            (FitTag::Tank, 0.3),
        ],
        ArchetypeId::Disengage => &[
            (FitTag::Utility, 0.8),
            (FitTag::Support, 0.5),
            (FitTag::Enchanter, 0.3),
        ],
        ArchetypeId::Frontline => &[
            (FitTag::Frontline, 1.0),
            (FitTag::Tank, 0.7),
            (FitTag::Durability, 0.6),
        ],
        ArchetypeId::Tank => &[
            (FitTag::Tank, 1.0),
            (FitTag::Durability, 0.8),
            (FitTag::Frontline, 0.6),
        ],
        ArchetypeId::Bruiser => &[
            (FitTag::Bruiser, 1.0),
            (FitTag::Frontline, 0.5),
            (FitTag::Durability, 0.4),
        ],
        ArchetypeId::Assassin => &[
            (FitTag::Assassin, 1.0),
            (FitTag::Burst, 0.6),
            (FitTag::Mobility, 0.4),
        ],
        ArchetypeId::Poke => &[(FitTag::Poke, 1.0), (FitTag::Mage, 0.5), (FitTag::Ap, 0.4)],
        ArchetypeId::Burst => &[
            (FitTag::Burst, 1.0),
            (FitTag::Mage, 0.5),
            (FitTag::Ap, 0.4),
            (FitTag::Assassin, 0.3),
        ],
        ArchetypeId::Dps => &[
            (FitTag::Dps, 1.0),
            (FitTag::Marksman, 0.5),
            (FitTag::AutoAttack, 0.4),
        ],
        ArchetypeId::Scaling => &[(FitTag::LateGame, 0.8), (FitTag::Dps, 0.3)],
        ArchetypeId::EarlyGame => &[(FitTag::EarlyGame, 1.0)],
        ArchetypeId::Peel => &[
            (FitTag::Support, 0.8),
            (FitTag::Enchanter, 0.7),
            (FitTag::Utility, 0.5),
        ],
        ArchetypeId::Utility => &[
            (FitTag::Support, 0.8),
            (FitTag::Utility, 0.8),
            (FitTag::Enchanter, 0.5),
        ],
        ArchetypeId::Catch => &[
            (FitTag::Catch, 0.9),
            (FitTag::Engage, 0.6),
            (FitTag::Assassin, 0.4),
            (FitTag::Mobility, 0.3),
        ],
        ArchetypeId::Splitpush => &[
            (FitTag::Splitpush, 1.0),
            (FitTag::AutoAttack, 0.5),
            (FitTag::Bruiser, 0.4),
            (FitTag::LateGame, 0.3),
        ],
        ArchetypeId::WomboCombo => &[(FitTag::Engage, 0.8), (FitTag::Mage, 0.4)],
        ArchetypeId::ZoneControl => &[
            (FitTag::Mage, 0.6),
            (FitTag::Utility, 0.4),
            (FitTag::Ap, 0.4),
        ],
        ArchetypeId::Sustain => &[
            (FitTag::Sustain, 0.8),
            (FitTag::HealShield, 0.4),
            (FitTag::Durability, 0.3),
        ],
        ArchetypeId::AntiTank => &[(FitTag::AntiTank, 1.0), (FitTag::Dps, 0.4)],
        ArchetypeId::Siege => &[(FitTag::Poke, 0.8), (FitTag::Mage, 0.4)],
    }
}

fn trait_fit(id: TraitId) -> &'static [(FitTag, f32)] {
    match id {
        TraitId::DamagePhysical => &[(FitTag::Ad, 1.0)],
        TraitId::DamageMagic => &[(FitTag::Ap, 1.0)],
        TraitId::DamageTrue => &[],
        TraitId::Range => &[],
        TraitId::Mobility => &[(FitTag::Mobility, 1.0)],
        TraitId::HardCc => &[(FitTag::Engage, 0.8)],
        TraitId::SoftCc => &[(FitTag::Utility, 0.4)],
        TraitId::Waveclear => &[],
        TraitId::ObjectiveDps => &[(FitTag::AutoAttack, 0.7)],
        TraitId::Durability => &[(FitTag::Durability, 1.0), (FitTag::Tank, 0.3)],
        TraitId::LaneSafety => &[],
        TraitId::EarlyPower => &[(FitTag::EarlyGame, 1.0)],
        TraitId::MidPower => &[],
        TraitId::LatePower => &[(FitTag::LateGame, 1.0)],
        TraitId::Aoe => &[(FitTag::Mage, 0.3)],
        TraitId::SingleTarget => &[(FitTag::Assassin, 0.3)],
        TraitId::GoldDependency => &[(FitTag::LateGame, 0.4)],
        TraitId::ExecutionDifficulty => &[],
    }
}

fn riot_fit(tag: &str) -> &'static [(FitTag, f32)] {
    match tag {
        "Tank" => &[
            (FitTag::Tank, 1.0),
            (FitTag::Frontline, 0.6),
            (FitTag::Durability, 0.7),
        ],
        "Fighter" => &[
            (FitTag::Bruiser, 1.0),
            (FitTag::Frontline, 0.5),
            (FitTag::Ad, 0.4),
        ],
        "Assassin" => &[
            (FitTag::Assassin, 1.0),
            (FitTag::Burst, 0.6),
            (FitTag::Ad, 0.4),
        ],
        "Mage" => &[(FitTag::Mage, 1.0), (FitTag::Ap, 0.8), (FitTag::Burst, 0.3)],
        "Marksman" => &[
            (FitTag::Marksman, 1.0),
            (FitTag::AutoAttack, 0.8),
            (FitTag::Ad, 0.6),
        ],
        "Support" => &[
            (FitTag::Support, 1.0),
            (FitTag::Enchanter, 0.4),
            (FitTag::Utility, 0.6),
        ],
        _ => &[],
    }
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
