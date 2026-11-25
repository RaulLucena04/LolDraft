import { Fragment, useEffect, useMemo, useState } from "react";
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
import { ProBuildsLink } from "./ProBuildsLink";

interface LoadoutPanelProps {
  champion: { key: number; slug: string; name: string; imagePath: string };
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

const CATEGORY_BORDER: Record<LoadoutCategory, string> = {
  damage: "border-red/25 hover:border-red/50",
  defense: "border-blue/25 hover:border-blue/50",
  utility: "border-success/25 hover:border-success/50",
};

const PROFILE_SOURCE_LABEL: Record<LoadoutProfileSource, string> = {
  profile: "Perfil estratégico",
  riot_tags: "Solo tags Riot",
  none: "Sin perfil ni tags",
};

function reasonSummary(item: LoadoutItem): string {
  const reason = item.reasons.find((entry) => entry.code.startsWith("threat:"));
  if (reason) return reason.label;
  const fit = item.reasons.find((entry) => entry.code.startsWith("fit:"));
  if (fit) return fit.label;
  return "Encaje funcional";
}

function reasonDetail(item: LoadoutItem): string {
  const reasons = item.reasons
    .filter((reason) => reason.code !== "function")
    .slice(0, 3)
    .map((reason) => `${reason.label}: ${reason.detail}`);
  return [item.function, ...reasons].join("\n\n");
}

function ItemIcon({
  item,
  size = "md",
}: {
  item: { icon: number; name: string };
  size?: "sm" | "md" | "lg";
}) {
  const [failed, setFailed] = useState(false);
  const dimension =
    size === "lg" ? "h-14 w-14" : size === "sm" ? "h-9 w-9" : "h-11 w-11";
  if (failed) {
    return (
      <span
        aria-hidden="true"
        className={`flex shrink-0 items-center justify-center rounded-md border border-line/90 bg-surface2 text-xs font-bold text-muted ${dimension}`}
      >
        {item.name.slice(0, 2)}
      </span>
    );
  }
  return (
    <img
      src={`/items/${item.icon}.png`}
      alt=""
      loading="lazy"
      decoding="async"
      draggable={false}
      onError={() => setFailed(true)}
      className={`shrink-0 rounded-md border border-line/80 bg-bg object-cover ${dimension}`}
    />
  );
}

function ScoreChip({ score }: { score: number }) {
  return (
    <span
      className="chip tabular border-line/80 text-muted"
      title="Encaje funcional 0-100. No es una probabilidad de victoria."
    >
      encaje {Math.round(score * 100)}
    </span>
  );
}

function ItemCard({ item, dense = false }: { item: LoadoutItem; dense?: boolean }) {
  return (
    <div
      className={`rounded-lg border bg-bg/40 p-2 ${CATEGORY_BORDER[item.category]}`}
      title={reasonDetail(item)}
    >
      <div className="flex items-start gap-2">
        <ItemIcon item={item} size={dense ? "sm" : "md"} />
        <div className="min-w-0 flex-1">
          <p className="text-[11px] font-semibold leading-tight text-ink">
            {item.name}
          </p>
          <div className="mt-0.5 flex flex-wrap items-center gap-1">
            <span className={`chip ${CATEGORY_TONE[item.category]}`}>
              {CATEGORY_LABEL[item.category]}
            </span>
            <ScoreChip score={item.score} />
          </div>
          {!dense && (
            <p className="mt-1 line-clamp-2 text-[10px] leading-snug text-muted">
              {item.function}
            </p>
          )}
        </div>
      </div>
      <p className="mt-1 truncate text-[10px] font-semibold text-gold/90">
        {reasonSummary(item)}
      </p>
    </div>
  );
}

function CoreItem({ item, last }: { item: LoadoutItem; last: boolean }) {
  return (
    <Fragment>
      <div
        className={`min-w-[92px] flex-1 rounded-lg border bg-bg/40 p-1.5 text-center ${CATEGORY_BORDER[item.category]}`}
        title={reasonDetail(item)}
      >
        <div className="flex justify-center">
          <ItemIcon item={item} size="lg" />
        </div>
        <p className="mt-1 text-[10px] font-semibold leading-tight text-ink">
          {item.name}
        </p>
        <p className="mt-0.5 truncate text-[9px] font-semibold text-gold/90">
          {reasonSummary(item)}
        </p>
        <p className="mt-0.5 text-[9px] uppercase tracking-wider text-muted">
          {CATEGORY_LABEL[item.category]}
        </p>
      </div>
      {!last && (
        <span
          aria-hidden="true"
          className="self-center text-base font-bold text-gold/50"
        >
          →
        </span>
      )}
    </Fragment>
  );
}

function ThreatChips({ threats }: { threats: LoadoutThreat[] }) {
  if (threats.length === 0) return null;
  return (
    <div className="mt-2 flex flex-wrap items-center gap-1">
      <span className="text-[9px] font-semibold uppercase tracking-wider text-muted">
        Amenazas
      </span>
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

function RuneIcon({ page, size = "md" }: { page: LoadoutRune; size?: "sm" | "md" | "lg" }) {
  const [failed, setFailed] = useState(false);
  const dimension =
    size === "lg" ? "h-14 w-14" : size === "sm" ? "h-9 w-9" : "h-11 w-11";
  if (failed) {
    return (
      <span
        aria-hidden="true"
        className={`flex shrink-0 items-center justify-center rounded-full border border-line/90 bg-surface2 text-xs font-bold text-muted ${dimension}`}
      >
        {page.keystone.slice(0, 1)}
      </span>
    );
  }
  return (
    <img
      src={`/runes/${page.keystoneId}.png`}
      alt=""
      loading="lazy"
      decoding="async"
      draggable={false}
      onError={() => setFailed(true)}
      className={`shrink-0 rounded-full border border-blue/40 bg-bg object-cover ${dimension}`}
    />
  );
}

function RuneChipList({ values }: { values: string[] }) {
  if (values.length === 0) return null;
  return (
    <div className="mt-1 flex flex-wrap gap-1">
      {values.map((value, index) => (
        <span key={`${value}-${index}`} className="chip border-line/80 text-ink/85">
          {value}
        </span>
      ))}
    </div>
  );
}

function PrimaryRune({ page }: { page: LoadoutRune }) {
  const reasons = page.reasons.filter((reason) => reason.code !== "function");
  return (
    <div className="rounded-lg border border-blue/30 bg-blue/5 p-2.5">
      <div className="flex items-start gap-2.5">
        <RuneIcon page={page} size="lg" />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-1">
            <span className="chip border-blue/40 text-blue-100">
              Página principal
            </span>
            <ScoreChip score={page.score} />
          </div>
          <p className="mt-1 text-[12px] font-bold leading-tight text-ink">
            {page.keystone}
          </p>
          <p className="text-[10px] text-muted">
            {page.primaryPath} · {page.name}
          </p>
        </div>
      </div>
      <div className="mt-2 grid gap-1.5 sm:grid-cols-2">
        <div>
          <p className="text-[9px] font-semibold uppercase tracking-wider text-gold/90">
            {page.primaryPath}
          </p>
          <RuneChipList values={page.primaryRunes} />
        </div>
        <div>
          <p className="text-[9px] font-semibold uppercase tracking-wider text-blue/90">
            {page.secondaryPath}
          </p>
          <RuneChipList values={page.secondaryRunes} />
        </div>
      </div>
      {page.shards.length > 0 && (
        <div className="mt-1.5">
          <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
            Fragmentos
          </p>
          <RuneChipList values={page.shards} />
        </div>
      )}
      <p className="mt-1.5 text-[10px] leading-snug text-muted">{page.function}</p>
      {reasons.length > 0 && (
        <ul className="mt-1 space-y-0.5">
          {reasons.slice(0, 2).map((reason) => (
            <li key={reason.code} className="text-[10px] leading-snug text-gold/90">
              <span className="font-semibold">{reason.label}: </span>
              {reason.detail}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function AlternativeRune({ page }: { page: LoadoutRune }) {
  return (
    <li className="rounded-lg border border-line/80 bg-bg/40">
      <details>
        <summary className="flex cursor-pointer list-none items-center gap-2 p-2 [&::-webkit-details-marker]:hidden">
          <RuneIcon page={page} size="sm" />
          <div className="min-w-0 flex-1">
            <p className="truncate text-[11px] font-semibold text-ink">
              {page.keystone}
            </p>
            <p className="truncate text-[10px] text-muted">
              {page.primaryPath} · {page.secondaryPath}
            </p>
          </div>
          <ScoreChip score={page.score} />
        </summary>
        <div className="border-t border-line/70 px-2.5 py-2">
          <p className="text-[10px] leading-snug text-muted">{page.function}</p>
          <div className="mt-1.5 grid gap-1.5 sm:grid-cols-2">
            <div>
              <p className="text-[9px] font-semibold uppercase tracking-wider text-gold/90">
                {page.primaryPath}
              </p>
              <RuneChipList values={page.primaryRunes} />
            </div>
            <div>
              <p className="text-[9px] font-semibold uppercase tracking-wider text-blue/90">
                {page.secondaryPath}
              </p>
              <RuneChipList values={page.secondaryRunes} />
            </div>
          </div>
          {page.shards.length > 0 && (
            <p className="mt-1 text-[10px] text-muted">
              Fragmentos: {page.shards.join(" · ")}
            </p>
          )}
        </div>
      </details>
    </li>
  );
}

/**
 * Objetos y runas recomendados **por función**, no por winrate.
 *
 * Se muestran en dos paneles separados para que la build y la página de runas
 * se lean de un vistazo. La base sale de una plantilla curada por campeón o,
 * si no existe, de reglas por función; los situacionales solo se activan con
 * la amenaza correspondiente. Nunca se presenta una regla como build observada.
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
      .loadoutRecommend(
        champion.key,
        role,
        enemyKey === "" ? [] : enemyKey.split(",").map(Number),
      )
      .then((result) => {
        if (!cancelled) setLoadout(result);
      })
      .catch((caught) => {
        if (cancelled) return;
        const commandError = caught as Partial<CommandError>;
        setLoadout(null);
        setError(
          commandError.message ??
            "No se pudo calcular la recomendación de objetos.",
        );
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [champion.key, role, enemyKey]);

  const primaryRune = loadout?.runes[0] ?? null;
  const alternativeRunes = loadout?.runes.slice(1) ?? [];

  return (
    <>
      <section className="panel border-gold/30 p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-1.5">
            <p className="eyebrow text-gold">Objetos</p>
            <span
              className="chip border-success/30 text-success"
              title="Catálogo de funciones escrito a mano; no son estadísticas de partidas"
            >
              Curado
            </span>
            {loadout?.buildSource === "template" && loadout.buildName && (
              <span className="chip border-gold/35 text-gold">
                {loadout.buildName}
              </span>
            )}
            {loadout?.buildSource === "rules" && (
              <span className="chip border-line text-muted">
                Reglas por función
              </span>
            )}
            <span className="chip">
              {context === "own" ? "Tu pick" : "Mejor pick"}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <ProBuildsLink
              championSlug={champion.slug}
              championName={champion.name}
            />
            <span className="text-[10px] uppercase tracking-wider text-muted">
              {roleText(role ?? loadout?.role ?? null)}
            </span>
          </div>
        </div>

        <div className="mt-2 flex items-center gap-2">
          <ChampionAvatar champion={champion} size="sm" />
          <div className="min-w-0">
            <p className="truncate text-sm font-bold text-ink">{champion.name}</p>
            <p className="text-[10px] text-muted">
              {loading
                ? "Cruzando función con el rival…"
                : loadout
                  ? `${loadout.buildName ?? "Sin plantilla"} · ${loadout.enemyPicks} rival(es) leídos`
                  : "Sin recomendación de objetos"}
            </p>
          </div>
        </div>

        {loadout && <ThreatChips threats={loadout.threats} />}

        {loading && (
          <p className="mt-3 text-[11px] text-muted">Calculando objetos…</p>
        )}

        {error && (
          <p className="mt-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[11px] text-danger">
            {error}
          </p>
        )}

        {!loading && !error && loadout && (
          <>
            {(loadout.starter || loadout.boots) && (
              <div className="mt-2.5 grid gap-2 sm:grid-cols-2">
                {loadout.starter && (
                  <div>
                    <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
                      Inicio
                    </p>
                    <div className="mt-1">
                      <ItemCard item={loadout.starter} />
                    </div>
                  </div>
                )}
                {loadout.boots && (
                  <div>
                    <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
                      Botas
                    </p>
                    <div className="mt-1">
                      <ItemCard item={loadout.boots} />
                    </div>
                  </div>
                )}
              </div>
            )}

            {loadout.core.length > 0 && (
              <div className="mt-2.5">
                <div className="flex items-center justify-between gap-2">
                  <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
                    Núcleo · orden de compra
                  </p>
                  <span className="text-[9px] text-muted/70">
                    Pasa el cursor para ver el porqué
                  </span>
                </div>
                <div className="mt-1.5 flex items-stretch gap-1 overflow-x-auto pb-1">
                  {loadout.core.map((item, index) => (
                    <CoreItem
                      key={item.id}
                      item={item}
                      last={index === loadout.core.length - 1}
                    />
                  ))}
                </div>
              </div>
            )}

            {loadout.situational.length > 0 && (
              <div className="mt-2.5">
                <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
                  Situacionales para este rival
                </p>
                <div className="mt-1 grid gap-1.5 sm:grid-cols-2">
                  {loadout.situational.map((item) => (
                    <ItemCard key={item.id} item={item} dense />
                  ))}
                </div>
              </div>
            )}

            {loadout.core.length === 0 && loadout.situational.length === 0 && (
              <p className="mt-2 rounded-lg border border-dashed border-line px-3 py-2 text-[11px] text-muted">
                No hay objetos con encaje suficiente para este campeón y rol.
              </p>
            )}

            <details className="mt-2.5 rounded-lg border border-line/70 bg-surface2/40 px-2.5 py-2">
              <summary className="cursor-pointer text-[10px] font-semibold uppercase tracking-wider text-muted">
                Notas y procedencia
              </summary>
              <ul className="mt-1.5 space-y-1">
                {loadout.caveats.map((caveat) => (
                  <li
                    key={caveat}
                    className="text-[10px] leading-relaxed text-muted"
                  >
                    {caveat}
                  </li>
                ))}
              </ul>
              <p className="mt-1.5 text-[10px] leading-relaxed text-muted/80">
                Catálogo {loadout.provenanceLabel.toLowerCase()} v
                {loadout.catalogVersion} · {loadout.catalogItemCount} objetos y{" "}
                {loadout.catalogRuneCount} páginas · reglas v
                {loadout.rulesVersion}.
                {loadout.provenanceDetail
                  ? ` ${loadout.provenanceDetail}.`
                  : ""}{" "}
                El encaje no es una probabilidad de victoria ni una build
                observada.
              </p>
            </details>
          </>
        )}
      </section>

      <section className="panel border-blue/25 p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-1.5">
            <p className="eyebrow text-blue">Runas</p>
            <span className="chip border-success/30 text-success">Curado</span>
            {loadout && loadout.runes.length > 0 && (
              <span className="chip border-blue/35 text-blue-100">
                {loadout.runes.length} página(s)
              </span>
            )}
          </div>
          <span className="text-[10px] uppercase tracking-wider text-muted">
            {loadout
              ? loadout.profileInferred
                ? "Perfil inferido"
                : PROFILE_SOURCE_LABEL[loadout.profileSource]
              : "Sin datos"}
          </span>
        </div>

        {loading && (
          <p className="mt-2 text-[11px] text-muted">Calculando runas…</p>
        )}

        {!loading && !error && loadout && (
          <>
            {primaryRune ? (
              <div className="mt-2">
                <PrimaryRune page={primaryRune} />
                {alternativeRunes.length > 0 && (
                  <div className="mt-2">
                    <p className="text-[9px] font-semibold uppercase tracking-wider text-muted">
                      Alternativas
                    </p>
                    <ul className="mt-1 space-y-1">
                      {alternativeRunes.map((page) => (
                        <AlternativeRune key={page.id} page={page} />
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            ) : (
              <p className="mt-2 rounded-lg border border-dashed border-line px-3 py-2 text-[11px] text-muted">
                No hay una página de runas con encaje suficiente para este
                campeón y rol.
              </p>
            )}
          </>
        )}

        {!loading && !error && !loadout && (
          <p className="mt-2 text-[11px] text-muted">
            Sin página de runas calculada.
          </p>
        )}

        {error && (
          <p className="mt-2 text-[11px] text-muted">
            Las runas no se pudieron calcular.
          </p>
        )}
      </section>
    </>
  );
}
