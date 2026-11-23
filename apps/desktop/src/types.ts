export type Side = "blue" | "red";
export type Role = "top" | "jungle" | "mid" | "adc" | "support";
export type ActionKind = "ban" | "pick";
export type StepStatus = "done" | "current" | "future";
export type RoleSource = "auto" | "manual" | "unassigned";

export interface RoleFit {
  role: Role;
  proficiency: number;
  confidence: number;
  source: string;
  isPrimary: boolean;
}

export interface Champion {
  key: number;
  slug: string;
  name: string;
  title: string;
  image: string;
  imagePath: string;
  tags: string[];
  roles: RoleFit[];
  primaryRole: Role | null;
  isFlex: boolean;
  profileStatus: "roles_only" | "identity_only";
  strategicProfile: "pending";
}

export interface ChampionRef {
  key: number;
  slug: string;
  name: string;
  title: string;
  image: string;
  imagePath: string;
}

export interface FormatStep {
  index: number;
  number: number;
  side: Side;
  kind: ActionKind;
}

export interface Format {
  id: string;
  name: string;
  steps: FormatStep[];
}

export interface DataStatus {
  patch: string | null;
  lastSyncMs: number | null;
  catalogSource: string | null;
  identitySource: string;
  rolesSource: string;
  championCount: number;
  championsWithRoles: number;
  championsWithoutRoles: number;
  taxonomyVersion: string;
  archetypeCount: number;
  traitCount: number;
  profileCount: number;
  activeProfileSet: string | null;
  strategicProfileStatus: "taxonomy_ready" | "profiles_loaded" | "ready";
  warnings: string[];
}

export interface SessionSummary {
  id: string;
  formatId: string;
  side: Side;
  actionCount: number;
  totalActions: number;
  updatedAt: number;
}

export interface Bootstrap {
  appName: string;
  appVersion: string;
  format: Format;
  data: DataStatus;
  champions: Champion[];
  sessions: SessionSummary[];
}

export interface DraftStepView {
  index: number;
  number: number;
  side: Side;
  kind: ActionKind;
  status: StepStatus;
  champion: ChampionRef | null;
}

export interface NextStep {
  index: number;
  number: number;
  side: Side;
  kind: ActionKind;
}

export interface PickView {
  champion: ChampionRef;
  role: Role | null;
  roleConfidence: number | null;
  roleSource: RoleSource;
  roleLocked: boolean;
  availableRoles: Role[];
}

export interface Assignment {
  roles: (Role | null)[];
  score: number;
  confidence: number;
  assigned: number;
}

export interface TeamView {
  side: Side;
  picks: PickView[];
  bans: ChampionRef[];
  assignment: Assignment;
}

export interface DraftView {
  id: string;
  formatId: string;
  formatName: string;
  side: Side;
  steps: DraftStepView[];
  nextStep: NextStep | null;
  isComplete: boolean;
  canUndo: boolean;
  canRedo: boolean;
  actionCount: number;
  totalActions: number;
  teams: {
    blue: TeamView;
    red: TeamView;
  };
}

export interface CommandError {
  code: string;
  message: string;
}

export interface ProfileSet {
  id: string;
  version: string;
  taxonomyVersion: string;
  patch: string | null;
  championCount: number;
  isActive: boolean;
  createdAt: number;
}

export type ProvenanceCode = "riot" | "curated" | "imported" | "inferred";

export interface ProfileProvenance {
  code: ProvenanceCode;
  label: string;
  detail: string | null;
}

export interface ProfileRating {
  id: string;
  label: string;
  value: number;
  confidence: number;
  provenance: ProfileProvenance;
}

export interface ProfileRoleRating {
  role: Role;
  proficiency: number;
  confidence: number;
  provenance: ProfileProvenance;
}

export interface ProfileDamageShares {
  physical: number;
  magic: number;
  trueDamage: number;
}

export interface ChampionProfileContext {
  hasDraft: boolean;
  side: Side | null;
  role: Role | null;
  picked: boolean;
  teammates: number;
}

export interface ChampionProfileDetail {
  key: number;
  name: string;
  slug: string;
  setId: string;
  setVersion: string;
  taxonomyVersion: string;
  patch: string | null;
  roles: ProfileRoleRating[];
  archetypes: ProfileRating[];
  traits: ProfileRating[];
  damage: ProfileDamageShares | null;
  knownCellRatio: number;
  observedCells: number;
  inferredCells: number;
  unknownCells: number;
  context: ChampionProfileContext;
  synergies: Synergy[];
}

export type CoverageKind = "archetype" | "trait";

export interface Coverage {
  id: string;
  kind: CoverageKind;
  label: string;
  valueLow: number;
  valueHigh: number;
  valueEstimate: number | null;
  confidence: number;
  known: number;
  total: number;
}

export interface DamageProfile {
  physical: number;
  magic: number;
  trueDamage: number;
  knownWeightRatio: number;
  confidence: number;
}

export interface Need {
  id: string;
  kind: CoverageKind;
  label: string;
  amount: number;
  status: "confirmed" | "possible";
  reasons: string[];
}

export interface WinCondition {
  id: string;
  label: string;
  score: number;
  confirmed: boolean;
}

export interface AnalysisSignal {
  id: string;
  kind: CoverageKind;
  label: string;
  valueLow: number;
  valueHigh: number;
  confidence: number;
}

export interface SynergyChampion {
  key: number;
  name: string;
}

export interface Synergy {
  id: string;
  label: string;
  champions: SynergyChampion[];
  scoreLow: number;
  scoreHigh: number;
  confidence: number;
  evidence: string[];
}

export interface Advantage {
  low: number;
  high: number;
  confidence: number;
  known: number;
  total: number;
  favored: Side | null;
}

export interface LaneMatchup {
  role: Role;
  blue: SynergyChampion;
  red: SynergyChampion;
  pressure: Advantage;
  allIn: Advantage;
}

export interface TeamMatchup {
  id: string;
  label: string;
  side: Side;
  attackLabel: string;
  defendLabel: string;
  advantageLow: number;
  advantageHigh: number;
  confidence: number;
  favored: Side | null;
}

export interface TeamAnalysis {
  side: Side;
  picks: number;
  skippedWithoutRole: number;
  partial: boolean;
  knowledge: number;
  confidence: number;
  archetypes: Coverage[];
  traits: Coverage[];
  damage: DamageProfile | null;
  needs: Need[];
  winConditions: WinCondition[];
  synergies: Synergy[];
  strengths: AnalysisSignal[];
  weaknesses: AnalysisSignal[];
}

export interface DraftAnalysis {
  paramsVersion: number;
  interactionsVersion: number;
  engineVersion: string;
  profileSetId: string;
  profileSetVersion: string;
  blue: TeamAnalysis;
  red: TeamAnalysis;
  laneMatchups: LaneMatchup[];
  matchups: TeamMatchup[];
}

export type RecommendationPhase =
  | "early_ban"
  | "early_pick"
  | "late_ban"
  | "late_pick";

export type RecommendationFeature =
  | "need"
  | "answer"
  | "synergy"
  | "counter"
  | "power"
  | "flex"
  | "balance"
  | "safety";

export type BanFeature =
  | "opp_need"
  | "opp_synergy"
  | "opp_power"
  | "opp_flex"
  | "own_threat"
  | "own_pick_value";

export type RecommendationRisk =
  | "redundant_damage"
  | "resource_conflict"
  | "no_frontline"
  | "scaling_conflict"
  | "unknown_profile"
  | "role_ambiguous";

export type NoDataReason =
  | "no_profiles"
  | "no_available_champions"
  | "no_viable_candidate";

export interface Contribution {
  feature: RecommendationFeature;
  value: number;
  weight: number;
  contribution: number;
}

export interface RiskContribution {
  kind: RecommendationRisk;
  amount: number;
  contribution: number;
}

export interface RecommendationReason {
  code: string;
  params: Record<string, unknown>;
}

export interface Recommendation {
  champion: ChampionRef;
  role: Role;
  score: number;
  scoreFraction: number;
  rankingScore: number;
  confidence: number;
  limitedData: boolean;
  contributions: Contribution[];
  risks: RiskContribution[];
  reasons: RecommendationReason[];
  missing: RecommendationFeature[];
  viableRoles: number;
}

export interface BanContribution {
  feature: BanFeature;
  value: number;
  weight: number;
  contribution: number;
}

export interface BanRecommendation {
  champion: ChampionRef;
  score: number;
  scoreFraction: number;
  rankingScore: number;
  confidence: number;
  limitedData: boolean;
  contributions: BanContribution[];
  risks: RiskContribution[];
  reasons: RecommendationReason[];
  missing: BanFeature[];
  ownPickValue: number;
}

/** Procedencia del dataset observado que alimentó la señal `power`. */
export interface StatsProvenance {
  id: string;
  version: string;
  queue: StatsQueue;
  patch: string;
  source: string;
  license: string;
  rows: number;
  minGames: number;
}

export interface RecommendationSet {
  phase: RecommendationPhase;
  side: Side;
  actionKind: ActionKind;
  actionNumber: number | null;
  engineVersion: string;
  profileSetId: string;
  profileSetVersion: string;
  paramsVersion: number;
  candidatesConsidered: number;
  picks: Recommendation[];
  bans: BanRecommendation[];
  noData: NoDataReason | null;
  limitedData: boolean;
  missingFeatures: RecommendationFeature[];
  missingBanFeatures: BanFeature[];
  /** `null` si no hay dataset observado activo para esta cola. */
  stats: StatsProvenance | null;
}

/** Estado manual de un draft de SoloQ: lo que el jugador ve en el cliente. */
export interface SoloqDraftInput {
  side: Side;
  /** Posición de pick dentro del equipo propio (1..=5). */
  slot: number;
  /** Rol reservado para este pick; `null` deja libres los demás. */
  role: Role | null;
  bans: number[];
  /** Picks aliados ya bloqueados, sin incluir el propio. */
  allyPicks: number[];
  enemyPicks: number[];
}

/** Draft leído del cliente de League of Legends. */
export interface SoloqImport {
  side: Side;
  slot: number;
  role: Role | null;
  ownPick: number | null;
  bans: number[];
  allyPicks: number[];
  enemyPicks: number[];
  timerPhase: string | null;
  unknownChampions: number[];
}

/** Cola del dataset. SoloQ y profesional nunca se mezclan. */
export type StatsQueue = "soloq" | "pro";

/** Dataset de estadísticas observadas importado por el usuario. */
export interface StatsDataset {
  id: string;
  version: string;
  queue: StatsQueue;
  patch: string;
  source: string;
  license: string;
  rowCount: number;
  createdAt: number;
  isActive: boolean;
}

/** Metadatos que acompañan a un CSV. Pueden venir también en comentarios `# clave: valor`. */
export interface StatsCsvMeta {
  id: string;
  version: string;
  source: string;
  license: string;
  queue: StatsQueue;
  patch: string;
}

/** Fila del CSV que no se pudo importar y por qué. */
export interface StatsDiscardedRow {
  line: number | null;
  reason: string;
}

/** Resultado de importar un CSV: dataset guardado y filas en cuarentena. */
export interface StatsImport {
  dataset: StatsDataset;
  discarded: StatsDiscardedRow[];
  summary: string;
}

/** Motivo por el que se eligió una build. */
export type BuildBasis = "observed_winrate" | "sample_size" | "only_build";

/** Página de runas de una build. */
export interface RunePage {
  primaryPath: string;
  keystone: string;
  primaryRunes: string[];
  secondaryPath: string;
  secondaryRunes: string[];
  shards: string[];
}

/** Build de un campeón en un rol, tal como viene del dataset. */
export interface ChampionBuild {
  champion: number;
  role: Role;
  name: string;
  starterItems: string[];
  coreItems: string[];
  situationalItems: string[];
  runes: RunePage;
  games: number;
  wins: number;
  shrunkWinrate: number;
  confidence: number;
  basis: BuildBasis;
}

/** Dataset de builds y runas importado por el usuario. */
export interface BuildDataset {
  id: string;
  version: string;
  queue: StatsQueue;
  patch: string;
  source: string;
  license: string;
  rowCount: number;
  createdAt: number;
  isActive: boolean;
}

/** Procedencia del dataset de builds que alimentó la build mostrada. */
export interface BuildsProvenance {
  id: string;
  version: string;
  queue: StatsQueue;
  patch: string;
  source: string;
  license: string;
  rows: number;
  minGames: number;
}

/** Build elegida y dataset del que sale. Ambos pueden ser `null`. */
export interface BuildLookup {
  build: ChampionBuild | null;
  dataset: BuildsProvenance | null;
}

/** Motivo legible de una sugerencia de objeto o runas. */
export interface LoadoutReason {
  code: string;
  label: string;
  detail: string;
}

export type LoadoutCategory = "damage" | "defense" | "utility";

/** Objeto sugerido por función y matchup. */
export interface LoadoutItem {
  id: string;
  name: string;
  category: LoadoutCategory;
  function: string;
  /** Índice de encaje 0..100; no es una probabilidad de victoria. */
  score: number;
  reasons: LoadoutReason[];
}

/** Página de runas sugerida por función y matchup. */
export interface LoadoutRune {
  id: string;
  name: string;
  primaryPath: string;
  keystone: string;
  primaryRunes: string[];
  secondaryPath: string;
  secondaryRunes: string[];
  shards: string[];
  function: string;
  score: number;
  reasons: LoadoutReason[];
}

/** Amenaza agregada del rival usada para puntuar el loadout. */
export interface LoadoutThreat {
  id: string;
  label: string;
  valueLow: number;
  valueHigh: number;
  confidence: number;
  severity: number;
  confirmed: boolean;
}

/** `profile` si hay perfil, `riot_tags` si solo hay tags, `none` sin datos. */
export type LoadoutProfileSource = "profile" | "riot_tags" | "none";

/** Objetos y runas derivados de reglas para el campeón frente al rival actual. */
export interface Loadout {
  champion: number;
  championName: string;
  role: Role | null;
  starter: LoadoutItem | null;
  boots: LoadoutItem | null;
  core: LoadoutItem[];
  situational: LoadoutItem[];
  runes: LoadoutRune[];
  threats: LoadoutThreat[];
  profileSource: LoadoutProfileSource;
  /** `true` si todas las celdas conocidas del perfil son inferidas (piloto). */
  profileInferred: boolean;
  caveats: string[];
  catalogVersion: string;
  rulesVersion: string;
  catalogItemCount: number;
  catalogRuneCount: number;
  provenanceLabel: string;
  provenanceDetail: string | null;
  enemyPicks: number;
  enemyConfidence: number;
}

/** Resultado de consultar el canal de actualizaciones. */
export interface UpdateCheck {
  currentVersion: string;
  latestVersion: string;
  available: boolean;
  /** `local` (carpeta) o `url`. */
  channel: "local" | "url";
  /** Carpeta o URL configurada, tal cual. */
  source: string;
  /** Ruta o URL del `latest.json` leído. */
  manifest: string;
  notes: string | null;
  /** Instalador resuelto, solo cuando hay una versión más nueva. */
  installer: string | null;
}

/** Resultado de lanzar una actualización. */
export interface UpdateInstall {
  /** `installing` si se descarga y ejecuta en silencio; `browser` si solo se abre la descarga. */
  action: "installing" | "browser";
  version: string;
  target: string;
  message: string;
}
