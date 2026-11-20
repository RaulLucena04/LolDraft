import { useEffect, useState } from "react";
import { api } from "../api";
import type {
  BuildBasis,
  BuildLookup,
  CommandError,
  Role,
  StatsQueue,
} from "../types";
import { formatPercent, roleText, STATS_QUEUE_LABEL } from "../lib/format";
import { ChampionAvatar } from "./ChampionAvatar";

interface ChampionBuildPanelProps {
  champion: { key: number; name: string; imagePath: string };
  role: Role | null;
  queue: StatsQueue;
  /** "own" cuando el campeón ya está bloqueado; "suggested" para el mejor pick. */
  context: "own" | "suggested";
}

const BASIS_LABEL: Record<BuildBasis, string> = {
  observed_winrate: "Mejor winrate del dataset",
  sample_size: "Muestra insuficiente · más jugada",
  only_build: "Build curada · sin muestra",
};

const BASIS_TONE: Record<BuildBasis, string> = {
  observed_winrate: "border-success/35 text-success",
  sample_size: "border-warning/35 text-warning",
  only_build: "border-line text-muted",
};

function ItemList({ title, items }: { title: string; items: string[] }) {
  if (items.length === 0) return null;
  return (
    <div>
      <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
        {title}
      </p>
      <div className="mt-1 flex flex-wrap gap-1">
        {items.map((item) => (
          <span key={item} className="chip border-line/80 text-ink/90">
            {item}
          </span>
        ))}
      </div>
    </div>
  );
}

function RuneBlock({ lookup }: { lookup: BuildLookup }) {
  const build = lookup.build;
  if (!build) return null;
  const runes = build.runes;  return (
    <div className="rounded-lg border border-line/80 bg-bg/40 p-2.5">
      <div className="grid gap-2 sm:grid-cols-2">
        <div>
          <p className="text-[9px] font-semibold uppercase tracking-wider text-gold/90">
            {runes.primaryPath} · {runes.keystone}
          </p>
          <ul className="mt-1 space-y-0.5">
            {runes.primaryRunes.map((rune) => (
              <li key={rune} className="text-[10px] text-ink/85">
                {rune}
              </li>
            ))}
          </ul>
        </div>
        <div>
          <p className="text-[9px] font-semibold uppercase tracking-wider text-blue/90">
            {runes.secondaryPath}
          </p>
          <ul className="mt-1 space-y-0.5">
            {runes.secondaryRunes.map((rune) => (
              <li key={rune} className="text-[10px] text-ink/85">
                {rune}
              </li>
            ))}
          </ul>
        </div>
      </div>
      {runes.shards.length > 0 && (
        <p className="mt-2 text-[10px] text-muted">
          Fragmentos: {runes.shards.join(" · ")}
        </p>
      )}
    </div>
  );
}

/**
 * Build y runas del campeón, con la muestra y la procedencia del dataset.
 *
 * Si no hay dataset activo para la cola o no hay fila para ese campeón, se
 * dice; nunca se rellena con una build inventada ni con el dataset de otra cola.
 */
export function ChampionBuildPanel({
  champion,
  role,
  queue,
  context,
}: ChampionBuildPanelProps) {
  const [lookup, setLookup] = useState<BuildLookup | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    api
      .buildsFor(champion.key, role, queue)
      .then((result) => {
        if (!cancelled) setLookup(result);
      })
      .catch((caught) => {
        if (cancelled) return;
        const commandError = caught as Partial<CommandError>;
        setLookup(null);
        setError(
          commandError.message ?? "No se pudo leer la build de este campeón.",
        );
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [champion.key, role, queue]);

  const build = lookup?.build ?? null;
  const dataset = lookup?.dataset ?? null;

  return (
    <section className="panel border-blue/20 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <p className="eyebrow text-blue">Build y runas</p>
          <span className="chip">{STATS_QUEUE_LABEL[queue]}</span>
          <span className="chip">
            {context === "own" ? "Tu pick" : "Mejor pick"}
          </span>
        </div>
        <span className="text-[10px] uppercase tracking-wider text-muted">
          {roleText(role ?? build?.role ?? null)}
        </span>
      </div>

      <div className="mt-2 flex items-center gap-2">
        <ChampionAvatar champion={champion} size="sm" />
        <div className="min-w-0">
          <p className="truncate text-sm font-bold text-ink">{champion.name}</p>
          <p className="text-[10px] text-muted">
            {build
              ? build.name
              : loading
                ? "Buscando build…"
                : "Sin build en el dataset"}
          </p>
        </div>
      </div>

      {loading && (
        <p className="mt-3 text-[11px] text-muted">Leyendo builds…</p>
      )}

      {error && (
        <p className="mt-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[11px] text-danger">
          {error}
        </p>
      )}

      {!loading && !error && !dataset && (
        <p className="mt-3 rounded-lg border border-dashed border-line px-3 py-2.5 text-[11px] leading-relaxed text-muted">
          No hay un dataset de builds activo para {STATS_QUEUE_LABEL[queue]}.
          Importa un JSON en Inicio → «Builds importadas» para ver objetos y
          runas aquí. No se usa el dataset de la otra cola.
        </p>
      )}

      {!loading && !error && dataset && !build && (
        <p className="mt-3 rounded-lg border border-dashed border-line px-3 py-2.5 text-[11px] leading-relaxed text-muted">
          El dataset {dataset.id} (parche {dataset.patch}) no tiene una build
          para {champion.name}
          {role ? ` en ${roleText(role).toLowerCase()}` : ""}. No se rellena
          con otra build ni con otro rol.
        </p>
      )}

      {build && (
        <>
          <div className="mt-2 flex flex-wrap items-center gap-1">
            <span className={`chip ${BASIS_TONE[build.basis]}`}>
              {BASIS_LABEL[build.basis]}
            </span>
            {build.games > 0 ? (
              <span className="chip tabular">
                {build.wins}/{build.games} · winrate ajustado{" "}
                {formatPercent(build.shrunkWinrate, 1)} · confianza{" "}
                {formatPercent(build.confidence, 0)}
              </span>
            ) : (
              <span className="chip border-warning/30 text-warning">
                Sin partidas observadas
              </span>
            )}
          </div>

          <div className="mt-2.5 space-y-2">
            <ItemList title="Inicio" items={build.starterItems} />
            <ItemList title="Núcleo" items={build.coreItems} />
            <ItemList title="Situacionales" items={build.situationalItems} />
            <RuneBlock lookup={lookup as BuildLookup} />
          </div>

          {dataset && (
            <p className="mt-2 text-[10px] leading-relaxed text-muted/80">
              Dataset {dataset.id} v{dataset.version} · parche {dataset.patch} ·{" "}
              {dataset.rows} builds · mínimo {dataset.minGames} partidas.
              Fuente: {dataset.source} · licencia: {dataset.license}.
            </p>
          )}
        </>
      )}
    </section>
  );
}
