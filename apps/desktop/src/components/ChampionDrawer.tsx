import { useEffect } from "react";
import type {
  Champion,
  ChampionProfileDetail,
  ProfileProvenance,
  ProfileRating,
} from "../types";
import { ROLE_LABEL, SIDE_LABEL, formatPercent } from "../lib/format";
import { ChampionAvatar } from "./ChampionAvatar";
import { ProfileBadge } from "./Badges";

interface ChampionDrawerProps {
  champion: Champion | null;
  profile: ChampionProfileDetail | null;
  profileLoading: boolean;
  profileError: string | null;
  busy: boolean;
  chooseBlockedReason: string | null;
  onClose: () => void;
  onChoose: (champion: Champion) => void;
}

export function ChampionDrawer({
  champion,
  profile,
  profileLoading,
  profileError,
  busy,
  chooseBlockedReason,
  onClose,
  onChoose,
}: ChampionDrawerProps) {
  useEffect(() => {
    if (!champion) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [champion, onClose]);

  if (!champion) return null;

  const hasRoles = champion.roles.length > 0;

  return (
    <div className="fixed inset-0 z-40 flex justify-end">
      <button
        type="button"
        aria-label="Cerrar detalle"
        onClick={onClose}
        className="absolute inset-0 bg-bg/70 backdrop-blur-[2px]"
      />
      <aside
        role="dialog"
        aria-modal="true"
        aria-labelledby="champion-detail-title"
        className="relative flex h-full w-full max-w-[430px] animate-rise flex-col overflow-y-auto border-l border-line bg-surface shadow-panel"
      >
        <div className="relative border-b border-line/70 px-5 pb-5 pt-4">
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-x-0 top-0 h-28 bg-gradient-to-b from-gold/10 to-transparent"
          />
          <div className="relative flex items-start justify-between">
            <p className="eyebrow">Detalle de campeón</p>
            <button
              type="button"
              onClick={onClose}
              className="rounded-md px-2 py-1 text-sm text-muted transition hover:bg-surface2 hover:text-ink"
              aria-label="Cerrar"
            >
              ×
            </button>
          </div>
          <div className="relative mt-4 flex items-center gap-4">
            <ChampionAvatar champion={champion} size="xl" />
            <div className="min-w-0">
              <h2
                id="champion-detail-title"
                className="truncate font-display text-2xl font-bold tracking-normal text-ink"
              >
                {champion.name}
              </h2>
              <p className="mt-0.5 truncate text-sm text-muted">{champion.title}</p>
              <div className="mt-2 flex flex-wrap gap-1">
                {champion.tags.map((tag) => (
                  <span key={tag} className="chip">
                    {tag}
                  </span>
                ))}
                <ProfileBadge
                  hasRoles={hasRoles}
                  hasProfile={profile !== null}
                  inferred={(profile?.inferredCells ?? 0) > 0}
                />
              </div>
            </div>
          </div>
        </div>

        <div className="flex-1 space-y-5 px-5 py-5">
          <section>
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted">
                Roles registrados
              </h3>
              <span className="text-[10px] text-muted/70">
                fuente: {champion.roles[0]?.source ?? "sin datos"}
              </span>
            </div>
            {hasRoles ? (
              <ul className="mt-3 space-y-2.5">
                {champion.roles.map((fit) => (
                  <li key={fit.role}>
                    <div className="flex items-center justify-between text-xs">
                      <span className="flex items-center gap-2 font-semibold text-ink">
                        {ROLE_LABEL[fit.role]}
                        {fit.isPrimary && (
                          <span className="rounded border border-gold/30 bg-gold/10 px-1 py-0.5 text-[9px] font-bold uppercase text-gold">
                            Principal
                          </span>
                        )}
                      </span>
                      <span className="tabular text-muted">
                        {formatPercent(fit.proficiency) ?? "—"}
                      </span>
                    </div>
                    <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface2">
                      <div
                        className="h-full rounded-full bg-gold/80"
                        style={{ width: `${fit.proficiency * 100}%` }}
                      />
                    </div>
                    <p className="mt-1 text-[10px] text-muted/70">
                      Confianza del dato:{" "}
                      {formatPercent(fit.confidence) ?? "desconocida"}
                    </p>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="mt-3 rounded-lg border border-warning/25 bg-warning/5 px-3 py-2.5">
                <p className="text-xs font-semibold text-warning">Rol pendiente</p>
                <p className="mt-1 text-[11px] leading-relaxed text-muted">
                  Data Dragon conoce al campeón, pero la semilla de roles todavía
                  no lo clasifica. No mostramos un rol inventado: el simulador lo
                  dejará sin asignar y podrás fijarlo manualmente cuando lo
                  selecciones.
                </p>
              </div>
            )}
          </section>

          <section className="rounded-lg border border-line/70 bg-bg/35 p-4">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted">
                Perfil estratégico
              </h3>
              {profile && (
                <span
                  className={`chip ${
                    profile.inferredCells > 0
                      ? "border-warning/30 text-warning"
                      : "border-success/30 text-success"
                  }`}
                >
                  {profile.inferredCells > 0
                    ? "Inferido · sin revisar"
                    : "Con fuente"}
                </span>
              )}
            </div>
            {profileLoading ? (
              <p className="mt-2 animate-pulse text-[11px] leading-relaxed text-muted">
                Cargando el perfil del conjunto activo…
              </p>
            ) : profileError ? (
              <p className="mt-2 rounded border border-warning/25 bg-warning/5 px-2.5 py-2 text-[11px] leading-relaxed text-warning/90">
                {profileError}
              </p>
            ) : profile ? (
              <ProfileDetails profile={profile} />
            ) : (
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                Sin perfil cargado. La UI no inventa celdas: si el campeón no
                está en el conjunto activo, se queda como desconocido.
              </p>
            )}
          </section>

          <section>
            <h3 className="text-xs font-bold uppercase tracking-wider text-muted">
              Identidad Riot
            </h3>
            <dl className="mt-2 space-y-1.5 text-xs">
              <div className="flex justify-between gap-3">
                <dt className="text-muted">Clave</dt>
                <dd className="tabular text-ink">{champion.key}</dd>
              </div>
              <div className="flex justify-between gap-3">
                <dt className="text-muted">Slug</dt>
                <dd className="truncate text-ink">{champion.slug}</dd>
              </div>
              <div className="flex justify-between gap-3">
                <dt className="text-muted">Imagen</dt>
                <dd className="truncate text-ink">{champion.image || "—"}</dd>
              </div>
            </dl>
          </section>
        </div>

        <div className="border-t border-line/70 px-5 py-4">
          {chooseBlockedReason ? (
            <p className="rounded-lg border border-line/70 bg-bg/40 px-3 py-2 text-center text-[11px] leading-relaxed text-muted">
              {chooseBlockedReason}
            </p>
          ) : (
            <>
              <button
                type="button"
                onClick={() => onChoose(champion)}
                disabled={busy}
                className="btn-gold w-full text-sm"
              >
                Elegir para el turno actual
              </button>
              <p className="mt-2 text-center text-[10px] text-muted/70">
                Todavía tendrás que confirmar la acción en el simulador.
              </p>
            </>
          )}
        </div>
      </aside>
    </div>
  );
}

function ProfileDetails({ profile }: { profile: ChampionProfileDetail }) {
  const known = profile.observedCells + profile.inferredCells;
  const total = Math.max(known + profile.unknownCells, 1);
  const allInferred = profile.observedCells === 0 && profile.inferredCells > 0;
  const sideLabel = profile.context.side
    ? SIDE_LABEL[profile.context.side]
    : null;

  return (
    <div className="mt-3 space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2 text-[10px] text-muted/80">
        <span className="truncate" title={profile.setId}>
          {profile.setId} · v{profile.setVersion}
        </span>
        <span>
          {profile.patch ? `parche ${profile.patch}` : "parche sin fijar"}
        </span>
      </div>

      <div>
        <div className="flex items-center justify-between text-[10px] text-muted">
          <span>Cobertura del perfil</span>
          <span className="tabular">
            {known} / {total} celdas
          </span>
        </div>
        <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-surface2">
          <div
            className="h-full rounded-full bg-gold/70"
            style={{ width: `${Math.max(1, profile.knownCellRatio * 100)}%` }}
          />
        </div>
        <p className="mt-1 text-[9px] leading-relaxed text-muted/70">
          {profile.observedCells} observadas · {profile.inferredCells} inferidas ·{" "}
          {profile.unknownCells} desconocidas
          {allInferred && " · las celdas conocidas son propuestas automáticas"}
        </p>
      </div>

      {profile.damage && (
        <div>
          <div className="flex items-center justify-between text-[10px] text-muted">
            <span>Reparto de daño</span>
            <span className="tabular">
              {formatPercent(profile.damage.physical) ?? "—"} físico
            </span>
          </div>
          <div className="mt-1 flex h-2 overflow-hidden rounded-full bg-surface2">
            <span
              className="bg-gold/80"
              style={{ width: `${profile.damage.physical * 100}%` }}
            />
            <span
              className="bg-blue/70"
              style={{ width: `${profile.damage.magic * 100}%` }}
            />
            <span
              className="bg-red/70"
              style={{ width: `${profile.damage.trueDamage * 100}%` }}
            />
          </div>
          <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-[9px] text-muted/80">
            <DamageShare
              color="bg-gold/80"
              label="Físico"
              value={profile.damage.physical}
            />
            <DamageShare
              color="bg-blue/70"
              label="Mágico"
              value={profile.damage.magic}
            />
            <DamageShare
              color="bg-red/70"
              label="Verdadero"
              value={profile.damage.trueDamage}
            />
          </div>
        </div>
      )}

      <ProfileRatingList
        title="Arquetipos"
        ratings={profile.archetypes}
        empty="Sin arquetipos conocidos en el set activo."
      />
      <ProfileRatingList
        title="Traits escalares"
        ratings={profile.traits}
        empty="Sin traits escalares conocidos en el set activo."
      />

      <section>
        <div className="flex items-center justify-between">
          <h4 className="text-[10px] font-bold uppercase tracking-wider text-muted">
            Sinergias con el equipo
          </h4>
          <span className="text-[9px] text-muted/60">derivadas</span>
        </div>
        {!profile.context.hasDraft ? (
          <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
            Abre un draft para evaluar parejas con tus picks. No se inventan
            compañeros.
          </p>
        ) : profile.context.role === null ? (
          <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
            Rol pendiente: sin un rol resuelto no se evalúan sinergias.
          </p>
        ) : profile.context.teammates === 0 ? (
          <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
            Todavía no hay compañeros en {sideLabel ?? "tu lado"} para evaluar
            parejas.
          </p>
        ) : profile.synergies.length === 0 ? (
          <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
            Sin parejas con evidencia suficiente. Una celda desconocida no crea
            una sinergia.
          </p>
        ) : (
          <>
            <p className="mt-1.5 text-[9px] leading-relaxed text-muted/70">
              Rol {profile.context.picked ? "asignado" : "de referencia"}:{" "}
              {ROLE_LABEL[profile.context.role]} ·{" "}
              {profile.context.teammates} compañero
              {profile.context.teammates === 1 ? "" : "s"} en{" "}
              {sideLabel ?? "tu lado"}.
            </p>
            <ul className="mt-1.5 space-y-1.5">
              {profile.synergies.map((synergy) => (
                <li
                  key={`${synergy.id}-${synergy.champions
                    .map((champion) => champion.key)
                    .join("-")}`}
                  className="rounded border border-line/70 bg-surface/50 px-2 py-1.5"
                >
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="text-[11px] font-semibold text-ink">
                      {synergy.label}
                    </span>
                    <span className="shrink-0 tabular text-[9px] text-muted">
                      {synergyRange(synergy.scoreLow, synergy.scoreHigh)}
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
          </>
        )}
        <p className="mt-1.5 text-[9px] leading-relaxed text-muted/60">
          Reglas sobre arquetipos y traits: no son datos observados ni
          probabilidades de victoria.
        </p>
      </section>
    </div>
  );
}

function ProfileRatingList({
  title,
  ratings,
  empty,
}: {
  title: string;
  ratings: ProfileRating[];
  empty: string;
}) {
  return (
    <div>
      <div className="flex items-center justify-between">
        <h4 className="text-[10px] font-bold uppercase tracking-wider text-muted">
          {title}
        </h4>
        <span className="text-[9px] text-muted/60">
          {ratings.length} con dato
        </span>
      </div>
      {ratings.length === 0 ? (
        <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
          {empty}
        </p>
      ) : (
        <ul className="mt-1.5 space-y-1.5">
          {ratings.map((rating) => (
            <li key={rating.id}>
              <div className="flex items-center justify-between gap-2 text-[11px]">
                <span className="flex min-w-0 items-center gap-1.5">
                  <span className="truncate text-ink">{rating.label}</span>
                  <ProvenanceTag provenance={rating.provenance} />
                </span>
                <span className="shrink-0 tabular text-muted">
                  {formatPercent(rating.value) ?? "—"}
                </span>
              </div>
              <div className="mt-0.5 h-1 overflow-hidden rounded-full bg-surface2">
                <div
                  className="h-full rounded-full bg-gold/70"
                  style={{
                    width: `${Math.max(1, rating.value * 100)}%`,
                  }}
                />
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ProvenanceTag({ provenance }: { provenance: ProfileProvenance }) {
  const inferred = provenance.code === "inferred";
  return (
    <span
      className={`shrink-0 rounded border px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider ${
        inferred
          ? "border-warning/30 bg-warning/10 text-warning"
          : "border-line bg-surface2 text-muted"
      }`}
      title={provenance.detail ?? provenance.label}
    >
      {provenance.label}
    </span>
  );
}

function DamageShare({
  color,
  label,
  value,
}: {
  color: string;
  label: string;
  value: number;
}) {
  return (
    <span className="inline-flex items-center gap-1">
      <span
        aria-hidden="true"
        className={`inline-block h-1.5 w-1.5 rounded-full ${color}`}
      />
      {label} {formatPercent(value) ?? "—"}
    </span>
  );
}

function synergyRange(low: number, high: number): string {
  const lowPercent = Math.round(low * 100);
  const highPercent = Math.round(high * 100);
  return lowPercent === highPercent
    ? `${lowPercent} %`
    : `${lowPercent}–${highPercent} %`;
}
