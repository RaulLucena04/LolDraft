import { useCallback, useEffect, useRef } from "react";
import type {
  Champion,
  DataStatus,
  DraftAnalysis,
  DraftView,
  Role,
} from "../types";
import { KIND_LABEL, SIDE_LABEL, formatRelative } from "../lib/format";
import { Logo } from "./Logo";
import { SideBadge, StatusPill } from "./Badges";
import { TeamPanel } from "./TeamPanel";
import { Timeline } from "./Timeline";
import { ChampionPool } from "./ChampionPool";
import { CompositionPanel } from "./CompositionPanel";

interface DraftScreenProps {
  draft: DraftView;
  champions: Champion[];
  data: DataStatus;
  busy: boolean;
  selected: Champion | null;
  onSelect: (champion: Champion | null) => void;
  onAction: (championKey: number) => Promise<boolean>;
  onUndo: () => void;
  onRedo: () => void;
  onSetRole: (championKey: number, role: Role | null) => void;
  onHome: () => void;
  onInspect: (championKey: number) => void;
  analysis: DraftAnalysis | null;
  analyzing: boolean;
  analysisError: string | null;
  onAnalyze: () => void;
}

export function DraftScreen({
  draft,
  champions,
  data,
  busy,
  selected,
  onSelect,
  onAction,
  onUndo,
  onRedo,
  onSetRole,
  onHome,
  onInspect,
  analysis,
  analyzing,
  analysisError,
  onAnalyze,
}: DraftScreenProps) {
  const searchRef = useRef<HTMLInputElement>(null);

  const next = draft.nextStep;
  const remaining = draft.totalActions - draft.actionCount;
  const profilesLoaded = data.profileCount > 0;
  const profileSetInferred =
    data.activeProfileSet?.toLowerCase().includes("inferred") ?? false;
  const profileSummary = profilesLoaded
    ? `${data.profileCount} ${profileSetInferred ? "inferidos" : "cargados"}`
    : "pendientes";

  const confirmSelection = useCallback(async () => {
    if (!selected || busy) return;
    const applied = await onAction(selected.key);
    if (applied) onSelect(null);
  }, [selected, busy, onAction, onSelect]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing =
        Boolean(target) &&
        (target!.tagName === "INPUT" ||
          target!.tagName === "TEXTAREA" ||
          target!.tagName === "SELECT" ||
          target!.isContentEditable);

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
        event.preventDefault();
        if (event.shiftKey) {
          onRedo();
        } else {
          onUndo();
        }
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") {
        event.preventDefault();
        onRedo();
        return;
      }
      if (!typing && event.key === "/") {
        event.preventDefault();
        searchRef.current?.focus();
        return;
      }
      if (event.key === "Escape") {
        onSelect(null);
        return;
      }
      if (!typing && event.key === "Enter" && selected) {
        event.preventDefault();
        void confirmSelection();
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selected, confirmSelection, onRedo, onUndo, onSelect]);

  return (
    <div className="flex min-h-screen flex-col">
      <header className="sticky top-0 z-20 border-b border-line/80 bg-bg/95 backdrop-blur">
        <div className="mx-auto flex w-full max-w-[1500px] flex-wrap items-center gap-3 px-4 py-2.5">
          <button
            type="button"
            onClick={onHome}
            className="btn-ghost px-2.5 py-1.5 text-xs"
            title="Volver al inicio"
          >
            ← Inicio
          </button>
          <Logo compact />
          <span className="hidden h-6 w-px bg-line sm:block" />
          <div className="hidden items-center gap-2 sm:flex">
            <span className="text-xs font-semibold text-ink">Simulador</span>
            <span className="chip">{draft.formatName}</span>
            {data.patch && <span className="chip">Parche {data.patch}</span>}
            <SideBadge side={draft.side} />
            <span className="text-[10px] text-muted">tu lado</span>
          </div>

          <div className="ml-auto flex items-center gap-1.5">
            <button
              type="button"
              onClick={onUndo}
              disabled={!draft.canUndo || busy}
              className="btn-ghost px-2.5 py-1.5 text-xs"
              title="Deshacer (Ctrl+Z)"
            >
              ↶ Deshacer
            </button>
            <button
              type="button"
              onClick={onRedo}
              disabled={!draft.canRedo || busy}
              className="btn-ghost px-2.5 py-1.5 text-xs"
              title="Rehacer (Ctrl+Y)"
            >
              ↷ Rehacer
            </button>
            <span className="chip tabular">
              {draft.actionCount} / {draft.totalActions}
            </span>
          </div>
        </div>
      </header>

      <main className="mx-auto w-full max-w-[1500px] flex-1 space-y-3 px-3 py-3">
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-line/70 bg-surface/60 px-3 py-2 text-[11px] text-muted">
          <StatusPill tone="ok">
            <span aria-hidden="true">✓</span> Simulación activa
          </StatusPill>
          <span>
            Fase 2: el draft es real y persistente. El análisis de composición
            está disponible con el piloto inferido; las sinergias curadas, los
            matchups observados y la revisión experta siguen pendientes.
          </span>
          <span className="ml-auto hidden text-[10px] text-muted/70 lg:block">
            Atajos: Ctrl+Z / Ctrl+Y · / buscar · ↵ confirmar
          </span>
        </div>

        <Timeline draft={draft} />

        <div className="grid grid-cols-1 items-stretch gap-3 lg:grid-cols-[minmax(250px,0.85fr)_minmax(380px,1.3fr)_minmax(250px,0.85fr)]">
          <TeamPanel
            team={draft.teams.blue}
            busy={busy}
            onSetRole={onSetRole}
            onInspect={onInspect}
          />

          <div className="flex min-w-0 flex-col gap-3">
            <div
              aria-live="polite"
              className={`panel border-l-4 px-4 py-2.5 ${
                next
                  ? next.side === "blue"
                    ? "border-l-blue bg-blueSoft/20"
                    : "border-l-red bg-redSoft/20"
                  : "border-l-success bg-success/5"
              }`}
            >
              {next ? (
                <>
                  <div className="flex items-center justify-between">
                    <p className="eyebrow">Turno actual</p>
                    <span className="tabular text-[10px] text-muted">
                      quedan {remaining}
                    </span>
                  </div>
                  <p className="mt-0.5 flex flex-wrap items-baseline gap-x-2 text-base font-extrabold tracking-tight text-ink">
                    {SIDE_LABEL[next.side]}{" "}
                    <span className="text-muted">
                      {KIND_LABEL[next.kind].toLowerCase()}
                    </span>
                    <span className="text-[11px] font-normal text-muted">
                      acción {next.number} de {draft.totalActions} · elige y
                      confirma
                    </span>
                  </p>
                </>
              ) : (
                <>
                  <p className="eyebrow text-success">Draft completo</p>
                  <p className="mt-0.5 flex flex-wrap items-baseline gap-x-2 text-base font-extrabold tracking-tight text-ink">
                    20 / 20 acciones
                    <span className="text-[11px] font-normal text-muted">
                      revisa la composición, ajusta roles o vuelve al inicio
                    </span>
                  </p>
                </>
              )}
            </div>

            <ChampionPool
              champions={champions}
              draft={draft}
              selected={selected}
              searchRef={searchRef}
              busy={busy}
              profileSummary={profileSummary}
              onSelect={onSelect}
              onInspect={onInspect}
              onConfirm={() => void confirmSelection()}
            />
          </div>

          <TeamPanel
            team={draft.teams.red}
            busy={busy}
            onSetRole={onSetRole}
            onInspect={onInspect}
          />
        </div>

        <CompositionPanel
          analysis={analysis}
          analyzing={analyzing}
          error={analysisError}
          hasProfiles={profilesLoaded}
          busy={busy}
          onAnalyze={onAnalyze}
        />

        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          <div className="panel border-gold/20 p-4">
            <div className="flex items-center justify-between">
              <p className="eyebrow text-gold">Recomendaciones</p>
              <span className="chip border-gold/30 text-gold">Fase 3</span>
            </div>
            <p className="mt-2 text-sm font-bold text-ink">
              Prioridad: composición, no winrate
            </p>
            <p className="mt-1.5 text-[11px] leading-relaxed text-muted">
              El motor puntuará necesidades, sinergias, counters, flexibilidad y
              contexto del draft. Cada recomendación explicará el porqué y su
              confianza.
            </p>
            <ul className="mt-3 space-y-1.5 text-[11px] text-muted/80">
              <li className="flex gap-2">
                <span aria-hidden="true" className="text-gold/70">
                  →
                </span>
                Sin barras de winrate como argumento principal.
              </li>
              <li className="flex gap-2">
                <span aria-hidden="true" className="text-gold/70">
                  →
                </span>
                Sin recomendaciones si faltan perfiles.
              </li>
            </ul>
          </div>

          <div className="panel p-4">
            <div className="flex items-center justify-between">
              <p className="eyebrow">Cobertura de roles</p>
              <span className="chip">Fase 1</span>
            </div>
            {(["blue", "red"] as const).map((side) => {
              const team = draft.teams[side];
              const total = team.picks.length;
              const assigned = team.assignment.assigned;
              const complete = total > 0 && assigned === total;
              return (
                <div key={side} className="mt-3">
                  <div className="flex items-center justify-between text-[11px]">
                    <span
                      className={
                        side === "blue" ? "text-blue-200" : "text-red-200"
                      }
                    >
                      {SIDE_LABEL[side]}
                    </span>
                    <span
                      className={`tabular ${
                        total === 0
                          ? "text-muted"
                          : complete
                            ? "text-success"
                            : "text-warning"
                      }`}
                    >
                      {assigned} / {total} roles
                    </span>
                  </div>
                  <div className="mt-1.5 flex gap-1">
                    {Array.from({ length: 5 }).map((_, index) => (
                      <span
                        key={index}
                        className={`h-1.5 flex-1 rounded-full ${
                          index < assigned
                            ? complete
                              ? "bg-success/80"
                              : "bg-gold/70"
                            : "bg-surface2"
                        }`}
                      />
                    ))}
                  </div>
                </div>
              );
            })}
            <p className="mt-3 text-[10px] leading-relaxed text-muted/70">
              Asignación óptima por permutaciones. Un pick sin rol se marca como
              pendiente; no se rellena con un rol inventado.
            </p>
          </div>

          <div className="panel p-4">
            <p className="eyebrow">Datos y perfiles</p>
            <dl className="mt-2 space-y-1.5 text-[11px]">
              <div className="flex justify-between gap-2">
                <dt className="text-muted">Parche</dt>
                <dd className="text-ink">{data.patch ?? "sin fijar"}</dd>
              </div>
              <div className="flex justify-between gap-2">
                <dt className="text-muted">Catálogo</dt>
                <dd className="tabular text-ink">
                  {data.championCount} campeones
                </dd>
              </div>
              <div className="flex justify-between gap-2">
                <dt className="text-muted">Roles</dt>
                <dd className="text-ink">{data.rolesSource}</dd>
              </div>
              <div className="flex justify-between gap-2">
                <dt className="text-muted">Taxonomía</dt>
                <dd className="text-ink">
                  v{data.taxonomyVersion} · {data.archetypeCount} arq. ·{" "}
                  {data.traitCount} traits
                </dd>
              </div>
              <div className="flex justify-between gap-2">
                <dt className="text-muted">Perfiles</dt>
                <dd className="text-ink">
                  {data.profileCount > 0
                    ? `${data.profileCount} · set ${data.activeProfileSet ?? "activo"}`
                    : "pendientes"}
                </dd>
              </div>
              <div className="flex justify-between gap-2">
                <dt className="text-muted">Sincronización</dt>
                <dd className="text-ink">
                  {data.lastSyncMs
                    ? formatRelative(data.lastSyncMs)
                    : "semilla local"}
                </dd>
              </div>
            </dl>
            {data.warnings.length > 0 && (
              <p className="mt-2 rounded border border-warning/20 bg-warning/5 px-2 py-1.5 text-[10px] leading-relaxed text-warning/90">
                {data.warnings[0]}
              </p>
            )}
          </div>
        </div>
      </main>

      <footer className="mx-auto w-full max-w-[1500px] px-4 pb-3 pt-1 text-[10px] text-muted/60">
        Datos de Riot Data Dragon y Meraki Analytics · No afiliado a Riot Games ·
        Puntuaciones de ajuste estratégico, no probabilidad de victoria
      </footer>
    </div>
  );
}
