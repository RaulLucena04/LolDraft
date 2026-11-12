import { useEffect } from "react";
import type { Champion } from "../types";
import { ROLE_LABEL, formatPercent } from "../lib/format";
import { ChampionAvatar } from "./ChampionAvatar";
import { ProfileBadge } from "./Badges";

interface ChampionDrawerProps {
  champion: Champion | null;
  busy: boolean;
  chooseBlockedReason: string | null;
  onClose: () => void;
  onChoose: (champion: Champion) => void;
}

export function ChampionDrawer({
  champion,
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
                className="truncate text-2xl font-extrabold tracking-tight text-ink"
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
                <ProfileBadge hasRoles={hasRoles} />
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
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted">
                Perfil estratégico
              </h3>
              <span className="chip border-warning/30 text-warning">
                Fase 2
              </span>
            </div>
            <p className="mt-2 text-[11px] leading-relaxed text-muted">
              Arquetipos, rasgos de daño, fortalezas, carencias y sinergias aún no
              están curados para este campeón. El motor no inventará esos valores.
            </p>
            <ul className="mt-3 space-y-1.5 text-[11px] text-muted/80">
              <li className="flex gap-2">
                <span aria-hidden="true" className="text-muted/50">1</span>
                Taxonomía v1 validada (20 arquetipos, 18 traits).
              </li>
              <li className="flex gap-2">
                <span aria-hidden="true" className="text-muted/50">2</span>
                Curación por lotes con fuente y confianza: en curso.
              </li>
              <li className="flex gap-2">
                <span aria-hidden="true" className="text-muted/50">3</span>
                Análisis con intervalos de desconocimiento.
              </li>
            </ul>
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
