use loldraft_domain::{ActionKind, Champion, DraftFormat, Role, Side};
use serde::Serialize;

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
    pub curated_profiles: usize,
    /// Estado honesto: la taxonomía ya está validada, pero los perfiles de
    /// campeón todavía no están curados.
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
