import type { ActionKind, Role, Side } from "../types";

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
