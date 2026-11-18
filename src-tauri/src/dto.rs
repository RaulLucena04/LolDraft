use loldraft_domain::{ActionKind, Champion, DraftFormat, Role, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto {
    pub app_name: String,
    pub app_version: String,
    pub format: FormatDto,
    pub data: DataStatusDto,
    pub champions: Vec<ChampionDto>,
    pub sessions: Vec<SessionSummaryDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatDto {
    pub id: String,
    pub name: String,
    pub steps: Vec<FormatStepDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatStepDto {
    pub index: usize,
    pub number: usize,
    pub side: Side,
    pub kind: ActionKind,
}

impl From<&DraftFormat> for FormatDto {
    fn from(format: &DraftFormat) -> Self {
        Self {
            id: format.id.clone(),
            name: format.name.clone(),
            steps: format
                .steps
                .iter()
                .enumerate()
                .map(|(index, step)| FormatStepDto {
                    index,
                    number: index + 1,
                    side: step.side,
                    kind: step.kind,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionDto {
    pub key: u16,
    pub slug: String,
    pub name: String,
    pub title: String,
    pub image: String,
    pub image_path: String,
    pub tags: Vec<String>,
    pub roles: Vec<RoleFitDto>,
    pub primary_role: Option<Role>,
    pub is_flex: bool,
    /// `roles_only` si tiene roles importados; `identity_only` si aún no.
    pub profile_status: &'static str,
    /// Los perfiles estratégicos se curan en la Fase 2. Nunca se inventan aquí.
    pub strategic_profile: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleFitDto {
    pub role: Role,
    pub proficiency: f32,
    pub confidence: f32,
    pub source: String,
    pub is_primary: bool,
}

impl From<&Champion> for ChampionDto {
    fn from(champion: &Champion) -> Self {
        let primary = champion.primary_role();
        Self {
            key: champion.key.0,
            slug: champion.slug.clone(),
            name: champion.name.clone(),
            title: champion.title.clone(),
            image: champion.image.clone(),
            image_path: champion_image_path(champion),
            tags: champion.tags.clone(),
            roles: champion
                .roles
                .iter()
                .map(|fit| RoleFitDto {
                    role: fit.role,
                    proficiency: fit.proficiency,
                    confidence: fit.confidence,
                    source: fit.source.clone(),
                    is_primary: primary == Some(fit.role),
                })
                .collect(),
            primary_role: primary,
            is_flex: champion.is_flex(),
            profile_status: if champion.roles.is_empty() {
                "identity_only"
            } else {
                "roles_only"
            },
            strategic_profile: "pending",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionRefDto {
    pub key: u16,
    pub slug: String,
    pub name: String,
    pub title: String,
    pub image: String,
    pub image_path: String,
}

impl From<&Champion> for ChampionRefDto {
    fn from(champion: &Champion) -> Self {
        Self {
            key: champion.key.0,
            slug: champion.slug.clone(),
            name: champion.name.clone(),
            title: champion.title.clone(),
            image: champion.image.clone(),
            image_path: champion_image_path(champion),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftViewDto {
    pub id: String,
    pub format_id: String,
    pub format_name: String,
    pub side: Side,
    pub steps: Vec<DraftStepViewDto>,
    pub next_step: Option<NextStepDto>,
    pub is_complete: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub action_count: usize,
    pub total_actions: usize,
    pub teams: TeamsDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftStepViewDto {
    pub index: usize,
    pub number: usize,
    pub side: Side,
    pub kind: ActionKind,
    /// `done`, `current` o `future`.
    pub status: &'static str,
    pub champion: Option<ChampionRefDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextStepDto {
    pub index: usize,
    pub number: usize,
    pub side: Side,
    pub kind: ActionKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamsDto {
    pub blue: TeamViewDto,
    pub red: TeamViewDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamViewDto {
    pub side: Side,
    pub picks: Vec<PickViewDto>,
    pub bans: Vec<ChampionRefDto>,
    pub assignment: AssignmentDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickViewDto {
    pub champion: ChampionRefDto,
    pub role: Option<Role>,
    /// Confianza del rol asignado. `None` cuando no hay rol; nunca un cero falso.
    pub role_confidence: Option<f32>,
    /// `auto`, `manual` o `unassigned`.
    pub role_source: &'static str,
    pub role_locked: bool,
    /// Roles que el catálogo reconoce para este campeón.
    pub available_roles: Vec<Role>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentDto {
    pub roles: Vec<Option<Role>>,
    pub score: f32,
    pub confidence: f32,
    pub assigned: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummaryDto {
    pub id: String,
    pub format_id: String,
    pub side: Side,
    pub action_count: usize,
    pub total_actions: usize,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStatusDto {
    pub patch: Option<String>,
    pub last_sync_ms: Option<i64>,
    pub catalog_source: Option<String>,
    pub identity_source: String,
    pub roles_source: String,
    pub champion_count: usize,
    pub champions_with_roles: usize,
    pub champions_without_roles: usize,
    pub taxonomy_version: String,
    pub archetype_count: usize,
    pub trait_count: usize,
    /// Perfiles del conjunto activo; no implica que estén revisados.
    pub profile_count: usize,
    pub active_profile_set: Option<String>,
    /// Estado honesto: taxonomía lista, perfiles cargados o análisis completo.
    pub strategic_profile_status: &'static str,
    pub warnings: Vec<String>,
}

pub fn champion_image_path(champion: &Champion) -> String {
    let file = if champion.image.is_empty() {
        format!("{}.png", champion.slug)
    } else {
        champion.image.clone()
    };
    format!("/champions/{file}")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSetDto {
    pub id: String,
    pub version: String,
    pub taxonomy_version: String,
    pub patch: Option<String>,
    pub champion_count: usize,
    pub is_active: bool,
    pub created_at: i64,
}

impl From<loldraft_data::ProfileSetSummary> for ProfileSetDto {
    fn from(summary: loldraft_data::ProfileSetSummary) -> Self {
        Self {
            id: summary.id,
            version: summary.version,
            taxonomy_version: summary.taxonomy_version,
            patch: summary.patch,
            champion_count: summary.champion_count,
            is_active: summary.is_active,
            created_at: summary.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageDto {
    pub id: String,
    pub kind: &'static str,
    pub label: String,
    pub value_low: f32,
    pub value_high: f32,
    pub value_estimate: Option<f32>,
    pub confidence: f32,
    pub known: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageProfileDto {
    pub physical: f32,
    pub magic: f32,
    pub true_damage: f32,
    pub known_weight_ratio: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeedDto {
    pub id: String,
    pub kind: &'static str,
    pub label: String,
    pub amount: f32,
    /// `confirmed` o `possible`.
    pub status: &'static str,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinConditionDto {
    pub id: &'static str,
    pub label: &'static str,
    pub score: f32,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalDto {
    pub id: String,
    pub kind: &'static str,
    pub label: String,
    pub value_low: f32,
    pub value_high: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynergyChampionDto {
    pub key: u16,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynergyDto {
    pub id: &'static str,
    pub label: &'static str,
    pub champions: Vec<SynergyChampionDto>,
    pub score_low: f32,
    pub score_high: f32,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvantageDto {
    pub low: f32,
    pub high: f32,
    pub confidence: f32,
    pub known: usize,
    pub total: usize,
    /// `None` cuando el intervalo cruza el cero.
    pub favored: Option<Side>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneMatchupDto {
    pub role: Role,
    pub blue: SynergyChampionDto,
    pub red: SynergyChampionDto,
    pub pressure: AdvantageDto,
    pub all_in: AdvantageDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMatchupDto {
    pub id: &'static str,
    pub label: &'static str,
    pub side: Side,
    pub attack_label: String,
    pub defend_label: String,
    pub advantage_low: f32,
    pub advantage_high: f32,
    pub confidence: f32,
    pub favored: Option<Side>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileProvenanceDto {
    /// Código estable: `riot`, `curated`, `imported` o `inferred`.
    pub code: &'static str,
    pub label: &'static str,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRatingDto {
    pub id: String,
    pub label: String,
    /// Solo se devuelven celdas conocidas; una celda desconocida no es un cero.
    pub value: f32,
    pub confidence: f32,
    pub provenance: ProfileProvenanceDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRoleDto {
    pub role: Role,
    pub proficiency: f32,
    pub confidence: f32,
    pub provenance: ProfileProvenanceDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDamageDto {
    pub physical: f32,
    pub magic: f32,
    pub true_damage: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionProfileContextDto {
    pub has_draft: bool,
    pub side: Option<Side>,
    /// Rol usado para evaluar sinergias; `None` si no hay draft o rol resuelto.
    pub role: Option<Role>,
    pub picked: bool,
    /// Compañeros evaluados, sin contar al propio campeón.
    pub teammates: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionProfileDto {
    pub key: u16,
    pub name: String,
    pub slug: String,
    pub set_id: String,
    pub set_version: String,
    pub taxonomy_version: String,
    pub patch: Option<String>,
    pub roles: Vec<ProfileRoleDto>,
    pub archetypes: Vec<ProfileRatingDto>,
    pub traits: Vec<ProfileRatingDto>,
    pub damage: Option<ProfileDamageDto>,
    pub known_cell_ratio: f32,
    pub observed_cells: usize,
    pub inferred_cells: usize,
    pub unknown_cells: usize,
    pub context: ChampionProfileContextDto,
    pub synergies: Vec<SynergyDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAnalysisDto {
    pub side: Side,
    pub picks: usize,
    pub skipped_without_role: usize,
    pub partial: bool,
    pub knowledge: f32,
    pub confidence: f32,
    pub archetypes: Vec<CoverageDto>,
    pub traits: Vec<CoverageDto>,
    pub damage: Option<DamageProfileDto>,
    pub needs: Vec<NeedDto>,
    pub win_conditions: Vec<WinConditionDto>,
    pub synergies: Vec<SynergyDto>,
    pub strengths: Vec<SignalDto>,
    pub weaknesses: Vec<SignalDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAnalysisDto {
    pub params_version: u32,
    pub interactions_version: u32,
    pub engine_version: String,
    pub profile_set_id: String,
    pub profile_set_version: String,
    pub blue: TeamAnalysisDto,
    pub red: TeamAnalysisDto,
    pub lane_matchups: Vec<LaneMatchupDto>,
    pub matchups: Vec<TeamMatchupDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionDto {
    pub feature: &'static str,
    pub value: f32,
    pub weight: f32,
    pub contribution: f32,
}

impl From<&loldraft_engine::Contribution> for ContributionDto {
    fn from(contribution: &loldraft_engine::Contribution) -> Self {
        Self {
            feature: contribution.feature.as_str(),
            value: contribution.value,
            weight: contribution.weight,
            contribution: contribution.contribution,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskContributionDto {
    pub kind: &'static str,
    pub amount: f32,
    pub contribution: f32,
}

impl From<&loldraft_engine::RiskContribution> for RiskContributionDto {
    fn from(risk: &loldraft_engine::RiskContribution) -> Self {
        Self {
            kind: risk.kind.as_str(),
            amount: risk.amount,
            contribution: risk.contribution,
        }
    }
}

/// Motivo tipado: `code` estable y parámetros serializados. La UI traduce el
/// código; el motor no envía frases libres.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasonDto {
    pub code: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationDto {
    pub champion: ChampionRefDto,
    pub role: Role,
    pub score: u8,
    pub score_fraction: f32,
    pub ranking_score: f32,
    pub confidence: f32,
    pub limited_data: bool,
    pub contributions: Vec<ContributionDto>,
    pub risks: Vec<RiskContributionDto>,
    pub reasons: Vec<ReasonDto>,
    pub missing: Vec<&'static str>,
    pub viable_roles: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BanContributionDto {
    pub feature: &'static str,
    pub value: f32,
    pub weight: f32,
    pub contribution: f32,
}

impl From<&loldraft_engine::BanContribution> for BanContributionDto {
    fn from(contribution: &loldraft_engine::BanContribution) -> Self {
        Self {
            feature: contribution.feature.as_str(),
            value: contribution.value,
            weight: contribution.weight,
            contribution: contribution.contribution,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BanRecommendationDto {
    pub champion: ChampionRefDto,
    pub score: u8,
    pub score_fraction: f32,
    pub ranking_score: f32,
    pub confidence: f32,
    pub limited_data: bool,
    pub contributions: Vec<BanContributionDto>,
    pub risks: Vec<RiskContributionDto>,
    pub reasons: Vec<ReasonDto>,
    pub missing: Vec<&'static str>,
    pub own_pick_value: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationSetDto {
    pub phase: &'static str,
    pub side: Side,
    pub action_kind: ActionKind,
    /// Número de la acción recomendada, 1..20. `None` si el draft terminó.
    pub action_number: Option<usize>,
    pub engine_version: String,
    pub profile_set_id: String,
    pub profile_set_version: String,
    pub params_version: u32,
    pub candidates_considered: usize,
    pub picks: Vec<RecommendationDto>,
    pub bans: Vec<BanRecommendationDto>,
    pub no_data: Option<&'static str>,
    pub limited_data: bool,
    pub missing_features: Vec<&'static str>,
    pub missing_ban_features: Vec<&'static str>,
}

/// Estado manual de un draft de SoloQ: lo que el jugador ve en el cliente.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoloqDraftInput {
    pub side: Side,
    /// Posición de pick dentro del equipo propio (1..=5).
    pub slot: u8,
    /// Rol reservado para este pick. `None` deja que el motor elija entre los libres.
    pub role: Option<Role>,
    #[serde(default)]
    pub bans: Vec<u16>,
    /// Picks aliados ya bloqueados, sin incluir el propio.
    #[serde(default)]
    pub ally_picks: Vec<u16>,
    #[serde(default)]
    pub enemy_picks: Vec<u16>,
}

/// Draft leído del cliente de League of Legends.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoloqImportDto {
    pub side: Side,
    pub slot: u8,
    pub role: Option<Role>,
    pub bans: Vec<u16>,
    pub ally_picks: Vec<u16>,
    pub enemy_picks: Vec<u16>,
    pub timer_phase: Option<String>,
    /// Campeones del cliente que no existen en el catálogo local.
    pub unknown_champions: Vec<u16>,
}

/// Resultado de consultar el canal de actualizaciones.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckDto {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    /// `local` (carpeta) o `url`.
    pub channel: &'static str,
    /// Carpeta o URL configurada, tal cual.
    pub source: String,
    /// Ruta o URL del `latest.json` leído.
    pub manifest: String,
    pub notes: Option<String>,
    /// Instalador resuelto, solo cuando hay una versión más nueva.
    pub installer: Option<String>,
}

/// Resultado de lanzar una actualización.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstallDto {
    /// `installer` si se ejecutó el instalador; `browser` si solo se abrió la
    /// descarga porque el canal es HTTP.
    pub action: &'static str,
    pub version: String,
    /// Instalador local o URL abierta.
    pub target: String,
    pub message: String,
}
