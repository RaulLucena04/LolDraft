import type { ActionKind, Role, Side, StatsQueue } from "../types";

export const SIDE_LABEL: Record<Side, string> = {
  blue: "Azul",
  red: "Rojo",
};

export const ROLE_LABEL: Record<Role, string> = {
  top: "Top",
  jungle: "Jungla",
  mid: "Mid",
  adc: "ADC",
  support: "Support",
};

export const ROLE_ORDER: Role[] = ["top", "jungle", "mid", "adc", "support"];

export const STATS_QUEUE_LABEL: Record<StatsQueue, string> = {
  soloq: "SoloQ",
  pro: "Profesional",
};

export const KIND_LABEL: Record<ActionKind, string> = {
  ban: "Ban",
  pick: "Pick",
};

export function sideText(side: Side): string {
  return SIDE_LABEL[side];
}

export function roleText(role: Role | null): string {
  return role ? ROLE_LABEL[role] : "Rol pendiente";
}

export function kindText(kind: ActionKind): string {
  return kind === "ban" ? "baneo" : "pick";
}

export function formatTimestamp(ms: number): string {
  const date = new Date(ms);
  return new Intl.DateTimeFormat("es-ES", {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatRelative(ms: number): string {
  const diff = Date.now() - ms;
  const minutes = Math.round(diff / 60_000);
  if (minutes < 1) return "ahora";
  if (minutes < 60) return `hace ${minutes} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `hace ${hours} h`;
  const days = Math.round(hours / 24);
  if (days < 7) return `hace ${days} d`;
  return formatTimestamp(ms);
}

export function formatPercent(value: number | null, digits = 0): string | null {
  if (value === null || Number.isNaN(value)) return null;
  return `${(value * 100).toFixed(digits)} %`;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/**
 * Enlace público de CounterStats para un campeón. La app no redistribuye esos
 * datos: solo abre la página para que el usuario consulte los enfrentamientos.
 *
 * CounterStats usa el nombre en minúsculas sin signos (`leesin`, `drmundo`,
 * `kaisa`). Nunu es la única excepción conocida (`nunu-willump`).
 */
export function counterstatsUrl(name: string): string {
  const normalized = name
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]/g, "");
  const slug = normalized === "nunuwillump" ? "nunu-willump" : normalized;
  return `https://www.counterstats.net/league-of-legends/${slug}`;
}
