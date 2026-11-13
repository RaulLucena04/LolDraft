import { invoke } from "@tauri-apps/api/core";
import type {
  Bootstrap,
  Champion,
  ChampionProfileDetail,
  CommandError,
  DraftAnalysis,
  DraftView,
  ProfileSet,
  Role,
  SessionSummary,
  Side,
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
  activateProfileSet: (id: string) =>
    call<ProfileSet[]>("profiles_activate", { id }),
  analysis: () => call<DraftAnalysis>("analysis_current"),
  championProfile: (championKey: number) =>
    call<ChampionProfileDetail>("profile_champion", { championKey }),
};

export { toCommandError };
