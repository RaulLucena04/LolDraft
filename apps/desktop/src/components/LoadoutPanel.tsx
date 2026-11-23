import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import type {
  CommandError,
  Loadout,
  LoadoutCategory,
  LoadoutItem,
  LoadoutProfileSource,
  LoadoutRune,
  LoadoutThreat,
  Role,
} from "../types";
import { formatPercent, roleText } from "../lib/format";
import { ChampionAvatar } from "./ChampionAvatar";

interface LoadoutPanelProps {
  champion: { key: number; name: string; imagePath: string };
  role: Role | null;
  enemyPicks: number[];
  /** "own" cuando el campeón ya está bloqueado; "suggested" para el mejor pick. */
  context: "own" | "suggested";
}

const CATEGORY_LABEL: Record<LoadoutCategory, string> = {
  damage: "Daño",
  defense: "Defensa",
  utility: "Utilidad",
};

const CATEGORY_TONE: Record<LoadoutCategory, string> = {
  damage: "border-red/35 text-red-200",
  defense: "border-blue/35 text-blue-100",
  utility: "border-success/35 text-success",
};

const PROFILE_SOURCE_LABEL: Record<LoadoutProfileSource, string> = {
  profile: "Perfil estratégico",
  riot_tags: "Solo tags Riot",
  none: "Sin perfil ni tags",
};

function ScoreChip({ score }: { score: number }) {
  return (
    <span className="chip tabular border-line/80 text-muted" title="Encaje 0-100, no es winrate">
      encaje {Math.round(score * 100)}
    </span>
  );
}

function ItemCard({ item }: { item: LoadoutItem }) {
  const reasons = item.reasons.filter((reason) => reason.code !== "function");
  return (
    <div className="rounded-lg border border-line/80 bg-bg/40 p-2">
      <div className="flex flex-wrap items-center gap-1">
        <span className="text-[11px] font-semibold text-ink">{item.name}</span>
        <span className={`chip ${CATEGORY_TONE[item.category]}`}>
          {CATEGORY_LABEL[item.category]}
        </span>
        <ScoreChip score={item.score} />
      </div>
      <p className="mt-1 text-[10px] leading-relaxed text-muted">{item.function}</p>
      {reasons.length > 0 && (
        <ul className="mt-1 space-y-0.5">
          {reasons.slice(0, 3).map((reason) => (
            <li key={reason.code} className="text-[10px] leading-relaxed text-gold/90">
              <span className="font-semibold">{reason.label}: </span>
              {reason.detail}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ItemGroup({ title, items }: { title: string; items: LoadoutItem[] }) {
  if (items.length === 0) return null;
  return (
    <div>
      <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
        {title}
      </p>
      <div className="mt-1 space-y-1.5">
        {items.map((item) => (
          <ItemCard key={item.id} item={item} />
        ))}
      </div>
    </div>
  );
}

function RuneCard({ page }: { page: LoadoutRune }) {
  return (
    <div className="rounded-lg border border-line/80 bg-bg/40 p-2">
      <div className="flex flex-wrap items-center gap-1">
        <span className="text-[11px] font-semibold text-ink">{page.name}</span>
        <ScoreChip score={page.score} />
      </div>
      <div className="mt-1 grid gap-1.5 sm:grid-cols-2">
        <div>
          <p className="text-[9px] font-semibold uppercase tracking-wider text-gold/90">
            {page.primaryPath} · {page.keystone}
          </p>
          <p className="mt-0.5 text-[10px] text-ink/85">
            {page.primaryRunes.join(" · ") || "—"}
          </p>
        </div>
        <div>
          <p className="text-[9px] font-semibold uppercase tracking-wider text-blue/90">
            {page.secondaryPath}
          </p>
          <p className="mt-0.5 text-[10px] text-ink/85">
            {page.secondaryRunes.join(" · ") || "—"}
          </p>
        </div>
      </div>
      {page.shards.length > 0 && (
        <p className="mt-1 text-[10px] text-muted">
          Fragmentos: {page.shards.join(" · ")}
        </p>
      )}
      <p className="mt-1 text-[10px] leading-relaxed text-muted">{page.function}</p>
      {page.reasons.some((reason) => reason.code !== "function") && (
        <ul className="mt-1 space-y-0.5">
          {page.reasons
            .filter((reason) => reason.code !== "function")
            .slice(0, 3)
            .map((reason) => (
              <li key={reason.code} className="text-[10px] leading-relaxed text-gold/90">
                <span className="font-semibold">{reason.label}: </span>
                {reason.detail}
              </li>
            ))}
        </ul>
      )}
    </div>
  );
}

function ThreatChips({ threats }: { threats: LoadoutThreat[] }) {
  if (threats.length === 0) return null;
  return (
    <div className="mt-2 flex flex-wrap gap-1">
      {threats.slice(0, 6).map((threat) => (
        <span
          key={threat.id}
          className={`chip ${
            threat.confirmed
              ? "border-warning/45 text-warning"
              : "border-line/80 text-muted"
          }`}
          title={`${threat.label}: nivel ${formatPercent(threat.severity, 0)} · datos ${formatPercent(threat.confidence, 0)}`}
        >
          {threat.label} · {formatPercent(threat.severity, 0)}
          {threat.confirmed ? "" : " · posible"}
        </span>
      ))}
    </div>
  );
}

/**
 * Objetos y runas recomendados **por función**, no por winrate.
 *
 * El catálogo es curado y se combina con las amenazas detectadas del rival y
 * con el perfil del campeón. Si no hay perfil se usan los tags de Riot y se
 * avisa; nunca se presenta una regla como si fuera una build observada.
 */
export function LoadoutPanel({
  champion,
  role,
  enemyPicks,
  context,
}: LoadoutPanelProps) {
  const [loadout, setLoadout] = useState<Loadout | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const enemyKey = useMemo(() => enemyPicks.join(","), [enemyPicks]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setLoadout(null);
    api
      .loadoutRecommend(champion.key, role, enemyKey === "" ? [] : enemyKey.split(",").map(Number))
      .then((result) => {
        if (!cancelled) setLoadout(result);
      })
      .catch((caught) => {
        if (cancelled) return;
        const commandError = caught as Partial<CommandError>;
        setLoadout(null);
        setError(
          commandError.message ?? "No se pudo calcular la recomendación de objetos.",
        );
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [champion.key, role, enemyKey]);

  return (
    <section className="panel border-blue/20 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <p className="eyebrow text-blue">Objetos y runas</p>
          <span className="chip border-success/30 text-success" title="Catálogo de funciones escrito a mano; no son estadísticas">
            Curado
          </span>
          {loadout?.profileInferred && (
            <span
              className="chip border-warning/40 text-warning"
              title="El perfil estratégico del campeón es un piloto inferido, no curado"
            >
              Perfil inferido
            </span>
          )}
          <span className="chip">
            {context === "own" ? "Tu pick" : "Mejor pick"}
          </span>
        </div>
        <span className="text-[10px] uppercase tracking-wider text-muted">
          {roleText(role ?? loadout?.role ?? null)}
        </span>
      </div>

      <div className="mt-2 flex items-center gap-2">
        <ChampionAvatar champion={champion} size="sm" />
        <div className="min-w-0">
          <p className="truncate text-sm font-bold text-ink">{champion.name}</p>
          <p className="text-[10px] text-muted">
            {loading
              ? "Cruzando función con el rival…"
              : loadout
                ? `${loadout.profileInferred ? "Perfil inferido" : PROFILE_SOURCE_LABEL[loadout.profileSource]} · ${loadout.enemyPicks} rival(es) leídos`
                : "Sin recomendación"}
          </p>
        </div>
      </div>

      {loading && (
        <p className="mt-3 text-[11px] text-muted">Calculando objetos y runas…</p>
      )}

      {error && (
        <p className="mt-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[11px] text-danger">
          {error}
        </p>
      )}

      {!loading && !error && loadout && (
        <>
          <ThreatChips threats={loadout.threats} />

          <div className="mt-2.5 space-y-2.5">
            {loadout.starter && (
              <ItemGroup title="Inicio" items={[loadout.starter]} />
            )}
            {loadout.boots && <ItemGroup title="Botas" items={[loadout.boots]} />}
            <ItemGroup title="Núcleo" items={loadout.core} />
            <ItemGroup title="Situacionales" items={loadout.situational} />
            {loadout.runes.length > 0 ? (
              <div>
                <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
                  Runas
                </p>
                <div className="mt-1 space-y-1.5">
                  {loadout.runes.map((page) => (
                    <RuneCard key={page.id} page={page} />
                  ))}
                </div>
              </div>
            ) : (
              <p className="rounded-lg border border-dashed border-line px-3 py-2 text-[11px] text-muted">
                No hay una página de runas con encaje suficiente para este
                campeón y rol.
              </p>
            )}
          </div>

          {loadout.caveats.length > 0 && (
            <ul className="mt-2.5 space-y-1 rounded-lg border border-line/70 bg-surface2/40 px-2.5 py-2">
              {loadout.caveats.map((caveat) => (
                <li key={caveat} className="text-[10px] leading-relaxed text-muted">
                  {caveat}
                </li>
              ))}
            </ul>
          )}

          <p className="mt-2 text-[10px] leading-relaxed text-muted/80">
            Catálogo {loadout.provenanceLabel.toLowerCase()} v
            {loadout.catalogVersion} · {loadout.catalogItemCount} objetos y{" "}
            {loadout.catalogRuneCount} páginas · reglas v{loadout.rulesVersion}.
            {loadout.provenanceDetail
              ? ` ${loadout.provenanceDetail}.`
              : ""}{" "}
            El encaje no es una probabilidad de victoria ni una build observada.
          </p>
        </>
      )}
    </section>
  );
}
