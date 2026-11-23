import { invoke } from "@tauri-apps/api/core";
import type {
  Bootstrap,
  BuildDataset,
  BuildLookup,
  Champion,
  ChampionProfileDetail,
  CommandError,
  DraftAnalysis,
  DraftView,
  Loadout,
  ProfileSet,
  RecommendationSet,
  Role,
  SessionSummary,
  Side,
  SoloqDraftInput,
  SoloqImport,
  StatsCsvMeta,
  StatsDataset,
  StatsImport,
  StatsQueue,
  UpdateCheck,
  UpdateInstall,
} from "./types";

function toCommandError(error: unknown): CommandError {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error
  ) {
    return error as CommandError;
  }
  if (error instanceof Error) {
    return { code: "unknown", message: error.message };
  }
  return { code: "unknown", message: String(error) };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toCommandError(error);
  }
}

export const api = {
  bootstrap: () => call<Bootstrap>("app_bootstrap"),
  champions: () => call<Champion[]>("champions_list"),
  dataStatus: () => call<Bootstrap["data"]>("data_status"),
  syncDdragon: () => call<Bootstrap["data"]>("data_sync_ddragon"),
  newDraft: (side: Side) => call<DraftView>("draft_new", { side }),
  action: (championKey: number) =>
    call<DraftView>("draft_action", { championKey }),
  undo: () => call<DraftView>("draft_undo"),
  redo: () => call<DraftView>("draft_redo"),
  setRole: (championKey: number, role: Role | null) =>
    call<DraftView>("draft_set_role", { championKey, role }),
  sessions: () => call<SessionSummary[]>("draft_list"),
  load: (id: string) => call<DraftView>("draft_load", { id }),
  deleteSession: (id: string) => call<boolean>("draft_delete", { id }),
  profiles: () => call<ProfileSet[]>("profiles_list"),
  importPilot: (activate = true) =>
    call<ProfileSet>("profiles_import_pilot", { activate }),
  importProfileSet: (setJson: string, activate = true) =>
    call<ProfileSet>("profiles_import_set", { setJson, activate }),
  activateProfileSet: (id: string) =>
    call<ProfileSet[]>("profiles_activate", { id }),
  analysis: () => call<DraftAnalysis>("analysis_current"),
  recommendations: () => call<RecommendationSet>("recommendations_current"),
  soloqRecommend: (draft: SoloqDraftInput) =>
    call<RecommendationSet>("soloq_recommend", { draft }),
  lcuChampSelect: () => call<SoloqImport>("lcu_champ_select"),
  stats: () => call<StatsDataset[]>("stats_list"),
  statsImportJson: (json: string, activate = true) =>
    call<StatsDataset>("stats_import_json", { json, activate }),
  statsImportCsv: (csv: string, meta: StatsCsvMeta, activate = true) =>
    call<StatsImport>("stats_import_csv", { csv, meta, activate }),
  statsActivate: (id: string) =>
    call<StatsDataset[]>("stats_activate", { id }),
  statsDelete: (id: string) => call<boolean>("stats_delete", { id }),
  builds: () => call<BuildDataset[]>("builds_list"),
  buildsImportJson: (json: string, activate = true) =>
    call<BuildDataset>("builds_import_json", { json, activate }),
  buildsActivate: (id: string) =>
    call<BuildDataset[]>("builds_activate", { id }),
  buildsDelete: (id: string) => call<boolean>("builds_delete", { id }),
  buildsFor: (champion: number, role: Role | null, queue: StatsQueue) =>
    call<BuildLookup>("builds_for", { champion, role, queue }),
  loadoutRecommend: (champion: number, role: Role | null, enemyPicks: number[]) =>
    call<Loadout>("loadout_recommend", { champion, role, enemyPicks }),
  championProfile: (championKey: number) =>
    call<ChampionProfileDetail>("profile_champion", { championKey }),
  settingGet: (key: string) => call<unknown | null>("settings_get", { key }),
  settingUpdate: (key: string, value: unknown) =>
    call<void>("settings_update", { key, value }),
  updateCheck: () => call<UpdateCheck>("update_check"),
  updateInstall: (version: string) =>
    call<UpdateInstall>("update_install", { version }),
};

export { toCommandError };
