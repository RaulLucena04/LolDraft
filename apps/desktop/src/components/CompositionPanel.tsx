import type {
  Coverage,
  DamageProfile,
  DraftAnalysis,
  TeamAnalysis,
} from "../types";
import { SideBadge, StatusPill } from "./Badges";

interface CompositionPanelProps {
  analysis: DraftAnalysis | null;
  analyzing: boolean;
  error: string | null;
  hasProfiles: boolean;
  busy: boolean;
  onAnalyze: () => void;
}

function percent(value: number): number {
  return Math.round(Math.max(0, Math.min(1, value)) * 100);
}

function clampPercent(value: number): number {
  return Math.max(0, Math.min(100, value));
}

function isInferredSet(analysis: DraftAnalysis): boolean {
  return (
    analysis.profileSetId.toLowerCase().includes("inferred") ||
    analysis.profileSetVersion.toLowerCase().includes("inferred")
  );
}

export function CompositionPanel({
  analysis,
  analyzing,
  error,
  hasProfiles,
  busy,
  onAnalyze,
}: CompositionPanelProps) {
  const inferredPilot = analysis ? isInferredSet(analysis) : false;

  return (
    <section className="panel border-gold/25 p-4" aria-live="polite">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="eyebrow text-gold">Análisis de composición</p>
            {analysis && (
              <span className="chip normal-case tracking-normal">
                Perfil {analysis.profileSetVersion}
              </span>
            )}
            {inferredPilot && (
              <StatusPill tone="warn">Inferido · revisión pendiente</StatusPill>
            )}
          </div>
          <h2 className="mt-1 text-sm font-bold text-ink">
            Lectura estratégica, no probabilidad de victoria
          </h2>
          <p className="mt-1 max-w-2xl text-[11px] leading-relaxed text-muted">
            Los intervalos muestran incertidumbre. La confianza nunca supera el
            conocimiento disponible de los perfiles y de la asignación de roles.
          </p>
        </div>

        <button
          type="button"
          onClick={onAnalyze}
          disabled={analyzing || busy || !hasProfiles}
          className="btn-gold px-4 py-2 text-xs"
        >
          {analyzing
            ? "Analizando…"
            : analysis
              ? "Recalcular análisis"
              : "Analizar composición"}
        </button>
      </div>

      {!hasProfiles ? (
        <div className="mt-4 rounded-lg border border-dashed border-warning/35 bg-warning/5 px-4 py-4">
          <p className="text-sm font-semibold text-warning">
            Sin perfiles estratégicos activos
          </p>
          <p className="mt-1 max-w-2xl text-[11px] leading-relaxed text-muted">
            Vuelve al inicio y pulsa «Cargar piloto inferido». El análisis no
            rellena arquetipos, daño ni carencias cuando faltan datos: los deja
            explícitamente desconocidos.
          </p>
        </div>
      ) : error ? (
        <div className="mt-4 rounded-lg border border-warning/35 bg-warning/5 px-4 py-4">
          <p className="text-sm font-semibold text-warning">
            No se pudo calcular el análisis
          </p>
          <p className="mt-1 text-[11px] leading-relaxed text-muted">{error}</p>
        </div>
      ) : !analysis ? (
        <div className="mt-4 rounded-lg border border-dashed border-line px-4 py-4">
          <p className="text-sm font-semibold text-ink">
            Análisis listo para ejecutarse
          </p>
          <p className="mt-1 max-w-2xl text-[11px] leading-relaxed text-muted">
            Se usarán los picks actuales de ambos lados y sus roles asignados.
            Los picks sin rol no se analizan con un rol inventado; se cuentan
            como omitidos.
          </p>
        </div>
      ) : (
        <>
          <div className="mt-4 grid grid-cols-1 gap-3 xl:grid-cols-2">
            <TeamAnalysisCard team={analysis.blue} />
            <TeamAnalysisCard team={analysis.red} />
          </div>
          <p className="mt-3 text-[10px] leading-relaxed text-muted/70">
            Motor {analysis.engineVersion} · parámetros v{analysis.paramsVersion}.
            Los arquetipos y traits son puntuaciones de ajuste estratégico, no
            winrates. {inferredPilot && "Este set usa celdas Inferred: no son datos observados ni curados."}
          </p>
        </>
      )}
    </section>
  );
}

function TeamAnalysisCard({ team }: { team: TeamAnalysis }) {
  const blue = team.side === "blue";
  const accentText = blue ? "text-blue-200" : "text-red-200";
  const accentBorder = blue ? "border-blue/35" : "border-red/35";
  const accentBg = blue ? "bg-blueSoft/25" : "bg-redSoft/25";
  const accentBar = blue ? "bg-blue/80" : "bg-red/80";

  if (team.picks === 0) {
    return (
      <article className={`rounded-xl border ${accentBorder} bg-bg/30 p-3.5`}>
        <div className="flex items-center gap-2">
          <SideBadge side={team.side} />
          <span className="text-[11px] text-muted">sin picks</span>
        </div>
        <p className="mt-3 text-[11px] leading-relaxed text-muted">
          Todavía no hay composición que analizar en este lado.
        </p>
      </article>
    );
  }

  return (
    <article className={`rounded-xl border ${accentBorder} bg-bg/30 p-3.5`}>
      <header className={`flex flex-wrap items-center gap-2 rounded-lg ${accentBg} px-2.5 py-2`}>
        <SideBadge side={team.side} />
        <span className="tabular text-[11px] text-ink">
          {team.picks} / 5 picks
        </span>
        <span className={`tabular text-[10px] ${accentText}`}>
          confianza {percent(team.confidence)} %
        </span>
        {team.partial ? (
          <span className="chip border-warning/40 text-warning">Parcial</span>
        ) : (
          <span className="chip border-success/35 text-success">Completa</span>
        )}
        <span className="ml-auto tabular text-[10px] text-muted">
          conocimiento {percent(team.knowledge)} %
        </span>
      </header>

      {team.skippedWithoutRole > 0 && (
        <p className="mt-2 rounded border border-warning/25 bg-warning/5 px-2 py-1 text-[10px] text-warning/90">
          {team.skippedWithoutRole} pick
          {team.skippedWithoutRole === 1 ? "" : "s"} sin rol asignado: no se
          analiza{team.skippedWithoutRole === 1 ? "" : "n"} con un rol inventado.
        </p>
      )}

      <section className="mt-3">
        <div className="flex items-center justify-between">
          <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
            Arquetipos dominantes
          </h3>
          <span className="text-[9px] text-muted/60">intervalo estimado</span>
        </div>
        <div className="mt-2 space-y-2">
          {team.archetypes.slice(0, 5).map((coverage) => (
            <CoverageRow
              key={coverage.id}
              coverage={coverage}
              barClass={accentBar}
            />
          ))}
        </div>
      </section>

      <section className="mt-3">
        <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
          Reparto de daño
        </h3>
        <div className="mt-2">
          <DamageBar damage={team.damage} />
        </div>
      </section>

      <section className="mt-3">
        <div className="flex items-center justify-between">
          <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
            Carencias a cubrir
          </h3>
          <span className="text-[9px] text-muted/60">según el rival actual</span>
        </div>
        {team.needs.length === 0 ? (
          <p className="mt-2 text-[10px] leading-relaxed text-muted/70">
            Sin carencias confirmadas con los datos actuales. No significa que la
            composición sea perfecta: significa que falta evidencia.
          </p>
        ) : (
          <div className="mt-2 space-y-2.5">
            {team.needs.slice(0, 5).map((need) => (
              <div key={need.id}>
                <div className="flex flex-wrap items-baseline justify-between gap-x-2">
                  <span className="text-[11px] font-semibold text-ink">
                    {need.label}
                  </span>
                  <span
                    className={`tabular text-[9px] ${
                      need.status === "confirmed"
                        ? "text-warning"
                        : "text-muted"
                    }`}
                  >
                    {need.status === "confirmed" ? "Confirmada" : "Posible"} ·{" "}
                    {percent(need.amount)} %
                  </span>
                </div>
                <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-surface2">
                  <div
                    className={`h-full rounded-full ${
                      need.status === "confirmed"
                        ? "bg-warning/80"
                        : "bg-warning/40"
                    }`}
                    style={{ width: `${clampPercent(percent(need.amount))}%` }}
                  />
                </div>
                {need.reasons.length > 0 && (
                  <p className="mt-0.5 text-[9px] leading-relaxed text-muted/65">
                    {need.reasons.join(" · ")}
                  </p>
                )}
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="mt-3">
        <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
          Condiciones de victoria
        </h3>
        {team.winConditions.length === 0 ? (
          <p className="mt-2 text-[10px] text-muted/70">
            Sin condiciones confirmadas todavía.
          </p>
        ) : (
          <div className="mt-2 space-y-1.5">
            {team.winConditions.slice(0, 3).map((condition) => (
              <div
                key={condition.id}
                className="flex items-center justify-between gap-2 rounded border border-line/70 bg-surface/50 px-2 py-1.5"
              >
                <span className="text-[11px] font-semibold text-ink">
                  {condition.label}
                </span>
                <span
                  className={`chip ${
                    condition.confirmed
                      ? "border-success/35 text-success"
                      : "text-muted"
                  }`}
                >
                  {condition.confirmed ? "Confirmada" : "Posible"} · ajuste{" "}
                  {condition.score.toFixed(2)}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>

      {team.strengths.length > 0 && (
        <section className="mt-3">
          <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
            Fortalezas detectadas
          </h3>
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {team.strengths.slice(0, 4).map((signal) => (
              <span key={signal.id} className="chip text-[10px]">
                {signal.label} ·{" "}
                {percent((signal.valueLow + signal.valueHigh) / 2)} %
              </span>
            ))}
          </div>
        </section>
      )}
    </article>
  );
}

function CoverageRow({
  coverage,
  barClass,
}: {
  coverage: Coverage;
  barClass: string;
}) {
  const low = clampPercent(percent(coverage.valueLow));
  const high = clampPercent(percent(coverage.valueHigh));
  // Un intervalo degenerado (por ejemplo 100–100) debe seguir siendo visible:
  // sin este ajuste la barra quedaría fuera del contenedor.
  const left = Math.min(low, 98);
  const width = Math.max(1, Math.min(high, 100) - left);

  return (
    <div>
      <div className="flex items-baseline justify-between gap-2">
        <span className="truncate text-[11px] text-ink">{coverage.label}</span>
        <span className="shrink-0 tabular text-[9px] text-muted">
          {coverage.valueEstimate != null
            ? `${percent(coverage.valueEstimate)} %`
            : `${low}–${high} %`}
        </span>
      </div>
      <div className="relative mt-1 h-1.5 overflow-hidden rounded-full bg-surface2">
        <div
          className={`absolute inset-y-0 rounded-full ${barClass}`}
          style={{ left: `${left}%`, width: `${width}%` }}
        />
        {coverage.valueEstimate != null && (
          <span
            aria-hidden="true"
            className="absolute top-1/2 h-2.5 w-0.5 -translate-y-1/2 rounded-full bg-ink"
            style={{ left: `${Math.min(percent(coverage.valueEstimate), 99)}%` }}
          />
        )}
      </div>
    </div>
  );
}

function DamageBar({ damage }: { damage: DamageProfile | null }) {
  if (!damage) {
    return (
      <p className="text-[10px] leading-relaxed text-muted/70">
        Sin datos de reparto de daño para estos picks. No se asume un 0 %.
      </p>
    );
  }

  const physical = percent(damage.physical);
  const magic = percent(damage.magic);
  const trueDamage = percent(damage.trueDamage);

  return (
    <div>
      <div className="flex h-2.5 overflow-hidden rounded-full bg-surface2">
        <span
          className="h-full bg-red"
          style={{ width: `${physical}%` }}
          title={`Físico ${physical} %`}
        />
        <span
          className="h-full bg-blue"
          style={{ width: `${magic}%` }}
          title={`Mágico ${magic} %`}
        />
        <span
          className="h-full bg-gold"
          style={{ width: `${trueDamage}%` }}
          title={`Verdadero ${trueDamage} %`}
        />
      </div>
      <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-[9px] text-muted">
        <span className="flex items-center gap-1">
          <span aria-hidden="true" className="h-1.5 w-1.5 rounded-full bg-red" />
          Físico {physical} %
        </span>
        <span className="flex items-center gap-1">
          <span aria-hidden="true" className="h-1.5 w-1.5 rounded-full bg-blue" />
          Mágico {magic} %
        </span>
        <span className="flex items-center gap-1">
          <span aria-hidden="true" className="h-1.5 w-1.5 rounded-full bg-gold" />
          Verdadero {trueDamage} %
        </span>
      </div>
      {damage.knownWeightRatio < 0.6 && (
        <p className="mt-1 text-[9px] leading-relaxed text-warning/85">
          Muestra parcial: solo el {percent(damage.knownWeightRatio)} % del peso
          de oro tiene daño conocido.
        </p>
      )}
    </div>
  );
}
