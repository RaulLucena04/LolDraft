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
  curatedProfiles: number;
  strategicProfileStatus: "pending";
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
