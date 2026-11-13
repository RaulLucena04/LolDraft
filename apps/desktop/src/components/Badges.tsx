import type { ReactNode } from "react";
import type { Role, Side } from "../types";
import { ROLE_LABEL, SIDE_LABEL } from "../lib/format";

export function SideBadge({ side }: { side: Side }) {
  const blue = side === "blue";
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-bold uppercase tracking-wide ${
        blue
          ? "border-blue/40 bg-blueSoft/60 text-blue-200"
          : "border-red/40 bg-redSoft/60 text-red-200"
      }`}
    >
      <span
        aria-hidden="true"
        className={`h-1.5 w-1.5 rounded-full ${blue ? "bg-blue" : "bg-red"}`}
      />
      {SIDE_LABEL[side]}
    </span>
  );
}

export function RoleBadge({
  role,
  muted = false,
}: {
  role: Role;
  muted?: boolean;
}) {
  return (
    <span
      className={`inline-flex items-center rounded border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
        muted
          ? "border-line bg-surface2 text-muted"
          : "border-gold/35 bg-gold/10 text-gold"
      }`}
    >
      {ROLE_LABEL[role]}
    </span>
  );
}

export function ConfidenceBadge({
  confidence,
}: {
  confidence: number | null;
}) {
  if (confidence === null) {
    return (
      <span className="chip border-warning/30 text-warning">Rol pendiente</span>
    );
  }
  const percent = Math.round(confidence * 100);
  const limited = confidence < 0.55;
  return (
    <span
      className={`chip tabular ${
        limited ? "border-warning/30 text-warning" : ""
      }`}
      title="Confianza del rol importado, no probabilidad de victoria"
    >
      {limited ? "Datos limitados" : "Rol"} {percent} %
    </span>
  );
}

export function ProfileBadge({
  hasRoles,
  hasProfile = false,
  inferred = false,
}: {
  hasRoles: boolean;
  hasProfile?: boolean;
  inferred?: boolean;
}) {
  if (!hasRoles) {
    return (
      <span className="chip border-warning/40 bg-warning/10 text-warning">
        Rol pendiente
      </span>
    );
  }
  if (!hasProfile) {
    return <span className="chip">Sin perfil en el set activo</span>;
  }
  if (inferred) {
    return (
      <span
        className="chip border-warning/40 bg-warning/10 text-warning"
        title="Celdas propuestas automáticamente; no son datos observados ni curados"
      >
        Perfil inferido
      </span>
    );
  }
  return (
    <span
      className="chip border-success/35 bg-success/10 text-success"
      title="Celdas con procedencia observada o curada"
    >
      Perfil con fuente
    </span>
  );
}

export function StatusPill({
  tone,
  children,
}: {
  tone: "ok" | "warn" | "info";
  children: ReactNode;
}) {
  const tones = {
    ok: "border-success/35 bg-success/10 text-success",
    warn: "border-warning/35 bg-warning/10 text-warning",
    info: "border-blue/35 bg-blue/10 text-blue-200",
  };
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-semibold ${tones[tone]}`}
    >
      {children}
    </span>
  );
}
