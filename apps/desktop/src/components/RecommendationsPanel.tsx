import type {
  BanFeature,
  BanRecommendation,
  NoDataReason,
  Recommendation,
  RecommendationFeature,
  RecommendationPhase,
  RecommendationReason,
  RecommendationRisk,
  RecommendationSet,
} from "../types";
import { ChampionAvatar } from "./ChampionAvatar";
import { RoleBadge, SideBadge } from "./Badges";

const PHASE_LABEL: Record<RecommendationPhase, string> = {
  early_ban: "Baneos iniciales",
  early_pick: "Picks iniciales",
  late_ban: "Baneos finales",
  late_pick: "Picks finales",
};

const PICK_FEATURE_LABEL: Record<RecommendationFeature, string> = {
  need: "Necesidad",
  answer: "Respuesta",
  synergy: "Sinergia",
  counter: "Counter",
  power: "Potencia observada",
  flex: "Flexibilidad",
  balance: "Equilibrio",
  safety: "Seguridad",
};

const BAN_FEATURE_LABEL: Record<BanFeature, string> = {
  opp_need: "Carencia rival",
  opp_synergy: "Sinergia rival",
  opp_power: "Potencia rival",
  opp_flex: "Flexibilidad rival",
  own_threat: "Amenaza propia",
  own_pick_value: "Valor para tu equipo",
};

const RISK_LABEL: Record<RecommendationRisk, string> = {
  redundant_damage: "Daño redundante",
  resource_conflict: "Conflicto de recursos",
  no_frontline: "Sin frontline",
  scaling_conflict: "Conflicto de escalado",
  unknown_profile: "Perfil incompleto",
  role_ambiguous: "Rol ambiguo",
};

const NO_DATA_LABEL: Record<NoDataReason, string> = {
  no_profiles: "No hay perfiles activos para puntuar candidatos.",
  no_available_champions:
    "No quedan campeones disponibles en el conjunto de perfiles activo.",
  no_viable_candidate:
    "Ningún candidato tiene datos suficientes para puntuarse sin inventar valores.",
};

interface RecommendationsPanelProps {
  recommendations: RecommendationSet | null;
  recommending: boolean;
  error: string | null;
  hasProfiles: boolean;
  draftComplete: boolean;
  onInspect: (championKey: number) => void;
  /** Etiqueta propia del contexto (SoloQ) en lugar de "Acción N de 20". */
  actionLabel?: string | null;
  /** Texto mostrado cuando aún no hay recomendación calculada. */
  emptyHint?: string;
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function percent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "—";
  return `${Math.round(clamp01(value) * 100)}%`;
}

function points(value: number): string {
  return (Math.round(value * 1000) / 10).toFixed(1);
}

function edge(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "—";
  const rounded = Math.round(value * 100) / 100;
  return `${rounded >= 0 ? "+" : ""}${rounded.toFixed(2)}`;
}

function readNumber(
  params: Record<string, unknown>,
  key: string,
): number | null {
  const value = params[key];
  return typeof value === "number" ? value : null;
}

function readString(
  params: Record<string, unknown>,
  key: string,
): string | null {
  const value = params[key];
  return typeof value === "string" ? value : null;
}

function reasonText(reason: RecommendationReason): string {
  const params = reason.params;
  const label = readString(params, "label");
  const before = readNumber(params, "before");
  const after = readNumber(params, "after");
  switch (reason.code) {
    case "fills_gap": {
      const need = readNumber(params, "need");
      return `Cubre ${label ?? "una carencia"}: ${percent(before)} → ${percent(
        after,
      )}${need !== null ? ` · necesidad ${percent(need)}` : ""}`;
    }
    case "answers_threat":
      return `Responde a una amenaza en ${label ?? "una dimensión"}: ${percent(
        before,
      )} → ${percent(after)}`;
    case "synergy": {
      const champion = readString(params, "championName") ?? "un aliado";
      const low = readNumber(params, "scoreLow");
      const high = readNumber(params, "scoreHigh");
      const range =
        low !== null && high !== null
          ? ` · ${low.toFixed(2)}–${high.toFixed(2)}`
          : "";
      return `Sinergia con ${champion}${label ? ` (${label})` : ""}${range}`;
    }
    case "counters_lane": {
      const champion = readString(params, "championName") ?? "el rival";
      const role = readString(params, "roleLabel");
      const low = readNumber(params, "edgeLow");
      const high = readNumber(params, "edgeHigh");
      const confirmed = low !== null && high !== null && low > 0;
      return `${confirmed ? "Counter de línea" : "Lectura de línea"} contra ${
        champion
      }${role ? ` (${role})` : ""} · ventaja ${edge(low)} a ${edge(high)}`;
    }
    case "flex_value": {
      const roles = readNumber(params, "roles");
      return roles !== null
        ? `Flexible: ${roles} roles compatibles`
        : "Flexible entre varios roles";
    }
    case "damage_balance":
      return `Equilibra ${label ?? "el reparto de daño"}: ${percent(
        before,
      )} → ${percent(after)}`;
    case "redundant_damage":
      return `Daño redundante en ${label ?? "una fuente"}: ${percent(
        readNumber(params, "share"),
      )} del daño conocido`;
    case "resource_conflict": {
      const allies = readNumber(params, "allies");
      return allies !== null
        ? `Conflicto de recursos con ${allies} ${
            allies === 1 ? "aliado" : "aliados"
          }`
        : "Conflicto de recursos con aliados";
    }
    case "no_frontline":
      return "El equipo sigue sin frontline";
    case "scaling_conflict":
      return "Conflicto de escalado: demasiados carries tardíos";
    case "unknown_profile":
      return `Perfil incompleto: conocimiento ${percent(
        readNumber(params, "knowledge"),
      )}`;
    case "role_ambiguous": {
      const roles = readNumber(params, "roles");
      return roles !== null
        ? `Rol ambiguo entre ${roles} roles`
        : "Rol ambiguo";
    }
    case "observed_power": {
      const queue = readString(params, "queueLabel") ?? "datos observados";
      const percentile = readNumber(params, "percentile");
      const games = readNumber(params, "games");
      const shrunk = readNumber(params, "shrunkWinrate");
      const ranking =
        percentile !== null
          ? `percentil ${Math.round(clamp01(percentile) * 100)}`
          : "percentil —";
      const sample = games !== null ? ` · n=${games}` : "";
      const winrate = shrunk !== null ? ` · winrate ajustado ${percent(shrunk)}` : "";
      return `Potencia observada en ${queue}: ${ranking}${winrate}${sample}`;
    }
    default:
      return reason.code;
  }
}

function MissingChips({
  labels,
}: {
  labels: string[];
}) {
  if (labels.length === 0) return null;
  return (
    <div>
      <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
        Sin datos (excluido y renormalizado)
      </p>
      <div className="mt-1 flex flex-wrap gap-1">
        {labels.map((label) => (
          <span key={label} className="chip border-line text-muted">
            {label}
          </span>
        ))}
      </div>
    </div>
  );
}

function ScoreBadge({
  score,
  tone = "gold",
}: {
  score: number;
  tone?: "gold" | "red";
}) {
  return (
    <div className="text-right">
      <p
        className={`tabular text-xl font-bold ${
          tone === "gold" ? "text-gold" : "text-red-200"
        }`}
      >
        {score}
      </p>
      <p className="text-[9px] uppercase tracking-wider text-muted">ajuste</p>
    </div>
  );
}

function DetailHeader({
  onInspect,
  championKey,
}: {
  onInspect: (championKey: number) => void;
  championKey: number;
}) {
  return (
    <div className="flex items-center justify-between">
      <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
        Desglose exacto
      </p>
      <button
        type="button"
        onClick={() => onInspect(championKey)}
        className="text-[10px] font-semibold text-gold/90 hover:text-gold"
      >
        Ver perfil
      </button>
    </div>
  );
}

function ReasonList({
  reasons,
  onInspect,
  championKey,
  opponentPerspective = false,
}: {
  reasons: RecommendationReason[];
  onInspect: (championKey: number) => void;
  championKey: number;
  opponentPerspective?: boolean;
}) {
  return (
    <div className="space-y-1.5">
      <DetailHeader onInspect={onInspect} championKey={championKey} />
      {opponentPerspective && (
        <p className="text-[10px] leading-snug text-muted/80">
          Motivos desde la perspectiva rival: lo que este campeón le aportaría
          y que el baneo elimina.
        </p>
      )}
      {reasons.length === 0 ? (
        <p className="text-[11px] text-muted/80">
          Sin motivos adicionales registrados.
        </p>
      ) : (
        <ul className="space-y-1 text-[11px] leading-snug text-ink/90">
          {reasons.map((reason, index) => (
            <li key={`${reason.code}-${index}`} className="flex gap-1.5">
              <span aria-hidden="true" className="text-gold/70">
                →
              </span>
              <span>{reasonText(reason)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ContributionBar({
  label,
  value,
  weight = null,
  contribution,
  negative = false,
  max,
}: {
  label: string;
  value: number;
  weight?: number | null;
  contribution: number;
  negative?: boolean;
  max: number;
}) {
  const width = max > 0 ? (Math.abs(contribution) / max) * 100 : 0;
  return (
    <li>
      <div className="flex items-baseline justify-between gap-2 text-[10px]">
        <span className="truncate text-ink/90">{label}</span>
        <span className="tabular shrink-0 text-muted">
          {percent(value)}
          {weight !== null ? ` · peso ${percent(Math.abs(weight))}` : ""} ·{" "}
          <span className={negative ? "text-red-300" : "text-gold/90"}>
            {contribution >= 0 ? "+" : "−"}
            {points(Math.abs(contribution))}
          </span>
        </span>
      </div>
      <div className="mt-0.5 h-1.5 overflow-hidden rounded-full bg-surface">
        <div
          className={`h-full rounded-full ${
            negative ? "bg-red/70" : "bg-gold/70"
          }`}
          style={{ width: `${Math.max(2, Math.min(100, width))}%` }}
        />
      </div>
    </li>
  );
}

function PickCard({
  index,
  pick,
  onInspect,
}: {
  index: number;
  pick: Recommendation;
  onInspect: (championKey: number) => void;
}) {
  const maxContribution = pick.contributions.reduce(
    (max, item) => Math.max(max, Math.abs(item.contribution)),
    0,
  );
  const missing = pick.missing.map((feature) => PICK_FEATURE_LABEL[feature]);
  return (
    <li className="rounded-lg border border-line bg-surface2/60">
      <details className="group">
        <summary className="flex cursor-pointer list-none items-center gap-2.5 p-2.5 [&::-webkit-details-marker]:hidden">
          <span className="tabular w-5 shrink-0 text-center text-[11px] font-bold text-muted">
            {index + 1}
          </span>
          <ChampionAvatar champion={pick.champion} size="sm" />
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-ink">
              {pick.champion.name}
            </p>
            <div className="mt-1 flex flex-wrap items-center gap-1">
              <RoleBadge role={pick.role} />
              <span
                className="chip tabular"
                title="Confianza del análisis, no probabilidad de victoria"
              >
                {percent(pick.confidence)} conf.
              </span>
              {pick.limitedData && (
                <span className="chip border-warning/35 text-warning">
                  Datos limitados
                </span>
              )}
              {pick.viableRoles > 1 && (
                <span className="chip">{pick.viableRoles} roles</span>
              )}
            </div>
          </div>
          <ScoreBadge score={pick.score} />
        </summary>
        <div className="space-y-2.5 border-t border-line/70 px-3 pb-3 pt-2.5">
          <ReasonList
            reasons={pick.reasons}
            onInspect={onInspect}
            championKey={pick.champion.key}
          />
          {pick.contributions.length > 0 && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                Contribuciones
              </p>
              <ul className="mt-1 space-y-1.5">
                {pick.contributions.map((item) => (
                  <ContributionBar
                    key={item.feature}
                    label={PICK_FEATURE_LABEL[item.feature]}
                    value={item.value}
                    weight={item.weight}
                    contribution={item.contribution}
                    max={maxContribution}
                  />
                ))}
              </ul>
            </div>
          )}
          {pick.risks.length > 0 && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                Riesgos
              </p>
              <ul className="mt-1 space-y-1.5">
                {pick.risks.map((risk) => (
                  <ContributionBar
                    key={risk.kind}
                    label={RISK_LABEL[risk.kind]}
                    value={risk.amount}
                    contribution={risk.contribution}
                    negative
                    max={maxContribution}
                  />
                ))}
              </ul>
            </div>
          )}
          <MissingChips labels={missing} />
        </div>
      </details>
    </li>
  );
}

function BanCard({
  index,
  ban,
  onInspect,
}: {
  index: number;
  ban: BanRecommendation;
  onInspect: (championKey: number) => void;
}) {
  const maxContribution = ban.contributions.reduce(
    (max, item) => Math.max(max, Math.abs(item.contribution)),
    0,
  );
  const missing = ban.missing.map((feature) => BAN_FEATURE_LABEL[feature]);
  return (
    <li className="rounded-lg border border-line bg-surface2/60">
      <details className="group">
        <summary className="flex cursor-pointer list-none items-center gap-2.5 p-2.5 [&::-webkit-details-marker]:hidden">
          <span className="tabular w-5 shrink-0 text-center text-[11px] font-bold text-muted">
            {index + 1}
          </span>
          <ChampionAvatar
            champion={ban.champion}
            size="sm"
            className="ring-1 ring-red/40"
          />
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-ink">
              {ban.champion.name}
            </p>
            <div className="mt-1 flex flex-wrap items-center gap-1">
              <span className="chip border-red/30 text-red-200">
                Baneo sugerido
              </span>
              <span
                className="chip tabular"
                title="Confianza del análisis, no probabilidad de victoria"
              >
                {percent(ban.confidence)} conf.
              </span>
              {ban.limitedData && (
                <span className="chip border-warning/35 text-warning">
                  Datos limitados
                </span>
              )}
            </div>
          </div>
          <ScoreBadge score={ban.score} tone="red" />
        </summary>
        <div className="space-y-2.5 border-t border-line/70 px-3 pb-3 pt-2.5">
          <ReasonList
            reasons={ban.reasons}
            onInspect={onInspect}
            championKey={ban.champion.key}
            opponentPerspective
          />
          {ban.contributions.length > 0 && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                Contribuciones (desde la perspectiva rival)
              </p>
              <ul className="mt-1 space-y-1.5">
                {ban.contributions.map((item) => (
                  <ContributionBar
                    key={item.feature}
                    label={BAN_FEATURE_LABEL[item.feature]}
                    value={item.value}
                    weight={item.weight}
                    contribution={item.contribution}
                    negative={item.contribution < 0}
                    max={maxContribution}
                  />
                ))}
              </ul>
            </div>
          )}
          {ban.risks.length > 0 && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                Riesgos
              </p>
              <ul className="mt-1 space-y-1.5">
                {ban.risks.map((risk) => (
                  <ContributionBar
                    key={risk.kind}
                    label={RISK_LABEL[risk.kind]}
                    value={risk.amount}
                    contribution={risk.contribution}
                    negative
                    max={maxContribution}
                  />
                ))}
              </ul>
            </div>
          )}
          <MissingChips labels={missing} />
        </div>
      </details>
    </li>
  );
}

export function RecommendationsPanel({
  recommendations,
  recommending,
  error,
  hasProfiles,
  draftComplete,
  onInspect,
  actionLabel = null,
  emptyHint = "Esperando el estado del draft para calcular recomendaciones.",
}: RecommendationsPanelProps) {
  const output = recommendations;
  const actionNumber = actionLabel === null ? output?.actionNumber ?? null : null;

  return (
    <section className="panel border-gold/20 p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <p className="eyebrow text-gold">Recomendaciones</p>
          {output && <span className="chip">{PHASE_LABEL[output.phase]}</span>}
          {output && <SideBadge side={output.side} />}
          {actionLabel !== null && (
            <span className="chip tabular">{actionLabel}</span>
          )}
          {actionNumber !== null && (
            <span className="chip tabular">Acción {actionNumber} de 20</span>
          )}
          {recommending && (
            <span className="chip border-gold/30 text-gold">Calculando…</span>
          )}
        </div>
        <p className="text-[10px] uppercase tracking-wider text-muted">
          Ajuste estratégico, no winrate
        </p>
      </div>

      <p className="mt-1.5 text-[11px] leading-relaxed text-muted">
        Índice 0–100 de encaje con la composición y el estado del draft. Las
        features sin datos se excluyen y los pesos restantes se renormalizan;
        nunca se rellenan con valores neutros.
      </p>

      {output?.stats && (
        <p className="mt-1.5 text-[10px] leading-relaxed text-muted/80">
          Potencia observada ·{" "}
          {output.stats.queue === "soloq" ? "SoloQ" : "Profesional"} ·{" "}
          {output.stats.id} v{output.stats.version} · parche{" "}
          {output.stats.patch} · {output.stats.rows} filas · mínimo{" "}
          {output.stats.minGames} partidas. Fuente: {output.stats.source} ·
          licencia: {output.stats.license}.
        </p>
      )}

      {output && !output.stats && (
        <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
          Sin dataset observado activo para esta cola: la señal de potencia queda
          fuera y los pesos se renormalizan. No se usa el dataset de otra cola.
        </p>
      )}

      {!hasProfiles && (
        <p className="mt-3 rounded-md border border-warning/30 bg-warning/5 px-3 py-2 text-[11px] text-warning">
          No hay un conjunto de perfiles activo. Carga y activa un conjunto para
          obtener recomendaciones: sin perfiles no se inventan puntuaciones.
        </p>
      )}

      {hasProfiles && draftComplete && (
        <p className="mt-3 rounded-md border border-line bg-surface2/60 px-3 py-2 text-[11px] text-muted">
          El draft está completo. No quedan acciones que recomendar.
        </p>
      )}

      {hasProfiles && !draftComplete && error && (
        <div className="mt-3 rounded-md border border-red/30 bg-red/5 px-3 py-2">
          <p className="text-[11px] text-red-200">{error}</p>
        </div>
      )}

      {hasProfiles && !draftComplete && !error && output?.noData && (
        <p className="mt-3 rounded-md border border-line bg-surface2/60 px-3 py-2 text-[11px] text-muted">
          {NO_DATA_LABEL[output.noData]}
        </p>
      )}

      {hasProfiles && !draftComplete && !output && recommending && (
        <div className="mt-3 space-y-2" aria-hidden="true">
          {Array.from({ length: 3 }).map((_, index) => (
            <div
              key={index}
              className="h-14 animate-pulse rounded-lg border border-line bg-surface2/40"
            />
          ))}
        </div>
      )}

      {hasProfiles && !draftComplete && !output && !recommending && !error && (
        <p className="mt-3 text-[11px] text-muted">{emptyHint}</p>
      )}

      {output && output.picks.length > 0 && (
        <div className="mt-3">
          <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
            Mejores picks · {output.candidatesConsidered} candidatos evaluados
          </p>
          <ul className="mt-1.5 space-y-1.5">
            {output.picks.map((pick, index) => (
              <PickCard
                key={`${pick.champion.key}-${pick.role}`}
                index={index}
                pick={pick}
                onInspect={onInspect}
              />
            ))}
          </ul>
        </div>
      )}

      {output && output.bans.length > 0 && (
        <div className="mt-3">
          <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
            Baneos sugeridos · {output.candidatesConsidered} candidatos
            evaluados
          </p>
          <ul className="mt-1.5 space-y-1.5">
            {output.bans.map((ban, index) => (
              <BanCard
                key={ban.champion.key}
                index={index}
                ban={ban}
                onInspect={onInspect}
              />
            ))}
          </ul>
        </div>
      )}

      {output &&
        (output.picks.length > 0 || output.bans.length > 0) && (
          <p className="mt-3 border-t border-line/70 pt-2 text-[10px] leading-relaxed text-muted/70">
            {output.actionKind === "ban"
              ? "Los baneos se puntúan desde la perspectiva del rival: se sugiere quitar lo que más le ayudaría. "
              : ""}
            La lista se diversifica por rol y arquetipo: una opción con puntuación
            algo menor puede adelantarse para no repetir rol o arquetipo. El
            orden no altera la puntuación mostrada.
          </p>
        )}
    </section>
  );
}
