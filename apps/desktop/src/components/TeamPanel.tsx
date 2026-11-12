import type { Role, TeamView } from "../types";
import { ROLE_LABEL, ROLE_ORDER, SIDE_LABEL } from "../lib/format";
import { ChampionAvatar } from "./ChampionAvatar";
import { ConfidenceBadge, RoleBadge, SideBadge } from "./Badges";

interface TeamPanelProps {
  team: TeamView;
  busy: boolean;
  onSetRole: (championKey: number, role: Role | null) => void;
  onInspect: (championKey: number) => void;
}

export function TeamPanel({
  team,
  busy,
  onSetRole,
  onInspect,
}: TeamPanelProps) {
  const blue = team.side === "blue";
  const picksByRole = new Map<Role, TeamView["picks"][number]>();
  for (const pick of team.picks) {
    if (pick.role) picksByRole.set(pick.role, pick);
  }
  const unassigned = team.picks.filter((pick) => !pick.role);

  return (
    <section
      className={`panel flex min-h-[420px] flex-col overflow-hidden border-t-2 ${
        blue ? "border-t-blue" : "border-t-red"
      }`}
    >
      <header
        className={`flex items-center justify-between gap-2 px-4 py-3 ${
          blue ? "bg-blueSoft/25" : "bg-redSoft/25"
        }`}
      >
        <div className="flex items-center gap-2">
          <SideBadge side={team.side} />
          <h2 className="text-sm font-bold uppercase tracking-wider text-ink">
            {SIDE_LABEL[team.side]}
          </h2>
        </div>
        <span className="chip tabular">
          {team.assignment.assigned} / {team.picks.length} roles
        </span>
      </header>

      <div className="flex flex-1 flex-col gap-2 p-3">
        {ROLE_ORDER.map((role) => {
          const pick = picksByRole.get(role);
          return (
            <div
              key={role}
              className={`rounded-lg border px-2.5 py-2 ${
                pick
                  ? "border-line bg-bg/45"
                  : "border-dashed border-line/60 bg-transparent"
              }`}
            >
              <div className="flex items-center gap-2.5">
                <RoleBadge role={role} muted={!pick} />
                {pick ? (
                  <>
                    <button
                      type="button"
                      onClick={() => onInspect(pick.champion.key)}
                      className="flex min-w-0 flex-1 items-center gap-2 text-left"
                      title={`Ver detalle de ${pick.champion.name}`}
                    >
                      <ChampionAvatar
                        champion={pick.champion}
                        size="sm"
                        className={blue ? "ring-1 ring-blue/40" : "ring-1 ring-red/40"}
                      />
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-semibold text-ink">
                          {pick.champion.name}
                        </span>
                        <span className="mt-0.5 flex flex-wrap items-center gap-1">
                          {pick.roleSource === "manual" ? (
                            <span className="chip border-gold/30 text-gold">
                              Rol fijado
                            </span>
                          ) : (
                            <ConfidenceBadge confidence={pick.roleConfidence} />
                          )}
                        </span>
                      </span>
                    </button>
                    <select
                      value={pick.roleLocked && pick.role ? pick.role : "auto"}
                      onChange={(event) =>
                        onSetRole(
                          pick.champion.key,
                          (event.target.value as Role | "auto") === "auto"
                            ? null
                            : (event.target.value as Role),
                        )
                      }
                      disabled={busy || pick.availableRoles.length < 2}
                      className="max-w-[92px] rounded-md border border-line bg-surface2 px-1.5 py-1 text-[10px] text-muted disabled:opacity-50"
                      title={
                        pick.availableRoles.length < 2
                          ? "Este campeón solo tiene un rol registrado"
                          : "Fijar rol manualmente"
                      }
                      aria-label={`Rol de ${pick.champion.name}`}
                    >
                      <option value="auto">Auto</option>
                      {pick.availableRoles.map((available) => (
                        <option key={available} value={available}>
                          {ROLE_LABEL[available]}
                        </option>
                      ))}
                    </select>
                  </>
                ) : (
                  <span className="flex flex-1 items-center gap-2 text-xs text-muted/70">
                    <span className="flex h-10 w-10 items-center justify-center rounded-md border border-line/50 text-base text-muted/40">
                      +
                    </span>
                    Sin pick
                  </span>
                )}
              </div>
            </div>
          );
        })}

        {unassigned.length > 0 && (
          <div className="rounded-lg border border-warning/25 bg-warning/5 px-2.5 py-2">
            <p className="text-[10px] font-semibold uppercase tracking-wider text-warning">
              Sin rol asignado
            </p>
            <div className="mt-1.5 flex flex-wrap gap-1.5">
              {unassigned.map((pick) => (
                <button
                  key={pick.champion.key}
                  type="button"
                  onClick={() => onInspect(pick.champion.key)}
                  className="flex items-center gap-1.5 rounded-md border border-line bg-surface2 px-1.5 py-1 text-[11px] text-ink"
                  title="Ver detalle"
                >
                  <ChampionAvatar champion={pick.champion} size="xs" />
                  {pick.champion.name}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="border-t border-line/70 px-3 py-2.5">
        <div className="flex items-center justify-between">
          <p className="eyebrow">Baneos</p>
          <span className="text-[10px] text-muted tabular">
            {team.bans.length} / 5
          </span>
        </div>
        <div className="mt-2 flex gap-1.5">
          {Array.from({ length: 5 }).map((_, index) => {
            const ban = team.bans[index];
            return ban ? (
              <button
                key={ban.key}
                type="button"
                onClick={() => onInspect(ban.key)}
                className="relative"
                title={`Baneado: ${ban.name}`}
              >
                <ChampionAvatar champion={ban} size="sm" dimmed />
                <span
                  aria-hidden="true"
                  className="absolute inset-0 flex items-center justify-center text-lg font-bold text-red-300/90"
                >
                  ×
                </span>
              </button>
            ) : (
              <span
                key={`empty-${index}`}
                className="flex h-10 w-10 items-center justify-center rounded-md border border-dashed border-line/60 text-[10px] text-muted/40"
              >
                —
              </span>
            );
          })}
        </div>
      </div>

      {team.picks.length > 0 && team.assignment.confidence < 0.25 && (
        <p className="border-t border-line/70 px-3 py-1.5 text-[10px] text-warning/80">
          Asignación ambigua: varios roles encajan igual de bien. Puedes fijar
          uno manualmente.
        </p>
      )}
    </section>
  );
}
