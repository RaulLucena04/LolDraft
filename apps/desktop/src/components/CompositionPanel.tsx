import type {
  Advantage,
  Coverage,
  DamageProfile,
  DraftAnalysis,
  TeamAnalysis,
} from "../types";
import { SIDE_LABEL } from "../lib/format";
import { RoleBadge, SideBadge, StatusPill } from "./Badges";

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
          <InteractionsPanel analysis={analysis} />
          <p className="mt-3 text-[10px] leading-relaxed text-muted/70">
            Motor {analysis.engineVersion} · parámetros v{analysis.paramsVersion} ·
            reglas de interacción v{analysis.interactionsVersion}. Los
            arquetipos y traits son puntuaciones de ajuste estratégico, no
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

      <section className="mt-3">
        <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
          Sinergias derivadas
        </h3>
        {team.synergies.length === 0 ? (
          <p className="mt-2 text-[10px] leading-relaxed text-muted/70">
            Sin parejas con evidencia suficiente. Las sinergias salen de reglas
            sobre arquetipos y traits; un dato desconocido no crea una sinergia.
          </p>
        ) : (
          <ul className="mt-2 space-y-1.5">
            {team.synergies.slice(0, 4).map((synergy) => (
              <li
                key={`${synergy.id}-${synergy.champions.map((champion) => champion.key).join("-")}`}
                className="rounded border border-line/70 bg-surface/50 px-2 py-1.5"
              >
                <div className="flex items-baseline justify-between gap-2">
                  <span className="text-[11px] font-semibold text-ink">
                    {synergy.label}
                  </span>
                  <span className="shrink-0 tabular text-[9px] text-muted">
                    {rangeText(synergy.scoreLow, synergy.scoreHigh)}
                  </span>
                </div>
                <p className="mt-0.5 text-[9px] leading-relaxed text-muted/70">
                  {synergy.champions
                    .map((champion) => champion.name)
                    .join(" + ")}
                  {" · "}
                  {synergy.evidence.join(" + ")}
                </p>
              </li>
            ))}
          </ul>
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

function InteractionsPanel({ analysis }: { analysis: DraftAnalysis }) {
  return (
    <div className="mt-3 grid grid-cols-1 gap-3 xl:grid-cols-2">
      <section className="rounded-xl border border-line/70 bg-bg/30 p-3.5">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
            Lectura de líneas
          </h3>
          <span className="text-[9px] text-muted/60">
            derivada de perfiles, no observada
          </span>
        </div>
        <p className="mt-1 text-[9px] leading-relaxed text-muted/65">
          La barra compara la presión y la amenaza de all-in de ambos picks: a la
          derecha, ventaja azul; a la izquierda, roja. Si el intervalo cruza el
          centro, no se afirma ninguna ventaja.
        </p>
        {analysis.laneMatchups.length === 0 ? (
          <p className="mt-2 text-[10px] leading-relaxed text-muted/70">
            Sin líneas con picks en ambos lados. No se empareja un campeón con un
            rival inventado.
          </p>
        ) : (
          <div className="mt-2 space-y-2">
            {analysis.laneMatchups.map((matchup) => (
              <div
                key={matchup.role}
                className="rounded-lg border border-line/70 bg-surface/40 px-2.5 py-2"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <RoleBadge role={matchup.role} muted />
                  <div className="flex items-center gap-1 text-[10px]">
                    <span className="font-semibold text-blue-200">
                      {matchup.blue.name}
                    </span>
                    <span className="text-muted/60">vs</span>
                    <span className="font-semibold text-red-200">
                      {matchup.red.name}
                    </span>
                  </div>
                </div>
                <div className="mt-2 grid grid-cols-2 gap-3">
                  <LaneMetric label="presión" advantage={matchup.pressure} />
                  <LaneMetric label="all-in" advantage={matchup.allIn} />
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="rounded-xl border border-line/70 bg-bg/30 p-3.5">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted">
            Ventajas estratégicas
          </h3>
          <span className="text-[9px] text-muted/60">
            herramienta contra herramienta
          </span>
        </div>
        <p className="mt-1 text-[9px] leading-relaxed text-muted/65">
          Cada fila compara una herramienta ofensiva de un lado con la respuesta
          del rival. No son enfrentamientos 1v1 ni probabilidades.
        </p>
        {analysis.matchups.length === 0 ? (
          <p className="mt-2 text-[10px] leading-relaxed text-muted/70">
            Sin ventajas claras con la evidencia actual. Un dato desconocido no
            produce una ventaja.
          </p>
        ) : (
          <ul className="mt-2 space-y-1.5">
            {analysis.matchups.map((matchup) => (
              <li
                key={`${matchup.id}-${matchup.side}`}
                className="rounded border border-line/70 bg-surface/50 px-2 py-1.5"
              >
                <div className="flex items-baseline justify-between gap-2">
                  <div className="flex min-w-0 items-center gap-1.5">
                    <span
                      aria-hidden="true"
                      className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                        matchup.side === "blue" ? "bg-blue" : "bg-red"
                      }`}
                    />
                    <span className="truncate text-[11px] font-semibold text-ink">
                      {matchup.label}
                    </span>
                  </div>
                  <span className="shrink-0 tabular text-[9px] text-muted">
                    {signedRangeValues(
                      matchup.advantageLow,
                      matchup.advantageHigh,
                    )}
                  </span>
                </div>
                <p className="mt-0.5 text-[9px] leading-relaxed text-muted/70">
                  {SIDE_LABEL[matchup.side]} · {matchup.attackLabel} frente a la
                  respuesta rival ({matchup.defendLabel.toLowerCase()})
                  {matchup.favored
                    ? ` · ${SIDE_LABEL[matchup.favored]} con ventaja`
                    : " · sin ventaja confirmada"}
                </p>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function LaneMetric({
  label,
  advantage,
}: {
  label: string;
  advantage: Advantage;
}) {
  const tone = !advantage.favored
    ? "text-muted/60"
    : advantage.favored === "blue"
      ? "text-blue-200"
      : "text-red-200";
  return (
    <div>
      <div className="flex items-baseline justify-between gap-1">
        <span className="text-[9px] uppercase tracking-wider text-muted/70">
          {label}
        </span>
        <span className={`tabular text-[9px] ${tone}`}>
          {signedAdvantageText(advantage)}
        </span>
      </div>
      <SignedBar advantage={advantage} />
    </div>
  );
}

function SignedBar({ advantage }: { advantage: Advantage }) {
  const toPercent = (value: number) => {
    const clamped = Math.max(-1, Math.min(1, value));
    return clampPercent(((clamped + 1) / 2) * 100);
  };
  // Un intervalo estrecho en el extremo debe seguir viéndose.
  const left = Math.min(toPercent(advantage.low), 98);
  const width = Math.max(1, toPercent(advantage.high) - left);
  const barClass = !advantage.favored
    ? "bg-muted/40"
    : advantage.favored === "blue"
      ? "bg-blue/80"
      : "bg-red/80";

  return (
    <div
      className="relative mt-1 h-1.5 overflow-hidden rounded-full bg-surface2"
      title={
        advantage.known === 0
          ? "Sin celdas conocidas para esta lectura"
          : `Confianza ${percent(advantage.confidence)} %`
      }
    >
      <span
        aria-hidden="true"
        className="absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-line"
      />
      <div
        className={`absolute inset-y-0 rounded-full ${barClass}`}
        style={{ left: `${left}%`, width: `${width}%` }}
      />
    </div>
  );
}

function rangeText(low: number, high: number): string {
  const lowPercent = percent(low);
  const highPercent = percent(high);
  return lowPercent === highPercent
    ? `${lowPercent} %`
    : `${lowPercent}–${highPercent} %`;
}

// La ventaja es una diferencia de cobertura (0..1), no un porcentaje de
// victoria: se muestra en puntos para no sugerir probabilidades.
function signedText(value: number): string {
  return `${value >= 0 ? "+" : "−"}${Math.abs(value).toFixed(2)}`;
}

function signedRangeValues(low: number, high: number): string {
  return `${signedText(low)}…${signedText(high)}`;
}

function signedAdvantageText(advantage: Advantage): string {
  return advantage.known === 0
    ? "sin datos"
    : signedRangeValues(advantage.low, advantage.high);
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
