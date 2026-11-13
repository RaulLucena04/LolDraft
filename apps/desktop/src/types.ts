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
