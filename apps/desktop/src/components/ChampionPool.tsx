import { useMemo, useState } from "react";
import type { RefObject } from "react";
import type { Champion, DraftView, Role } from "../types";
import { KIND_LABEL, ROLE_LABEL, ROLE_ORDER } from "../lib/format";
import { ChampionAvatar } from "./ChampionAvatar";

interface ChampionPoolProps {
  champions: Champion[];
  draft: DraftView;
  selected: Champion | null;
  searchRef: RefObject<HTMLInputElement | null>;
  busy: boolean;
  profileSummary: string;
  onSelect: (champion: Champion | null) => void;
  onInspect: (championKey: number) => void;
  onConfirm: () => void;
}

function normalize(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

export function ChampionPool({
  champions,
  draft,
  selected,
  searchRef,
  busy,
  profileSummary,
  onSelect,
  onInspect,
  onConfirm,
}: ChampionPoolProps) {
  const [query, setQuery] = useState("");
  const [roleFilter, setRoleFilter] = useState<Role | "all">("all");
  const [onlyAvailable, setOnlyAvailable] = useState(true);

  const used = useMemo(() => {
    const map = new Map<number, { side: "blue" | "red"; kind: "ban" | "pick" }>();
    for (const step of draft.steps) {
      if (step.champion && step.status === "done") {
        map.set(step.champion.key, { side: step.side, kind: step.kind });
      }
    }
    return map;
  }, [draft.steps]);

  const filtered = useMemo(() => {
    const needle = normalize(query.trim());
    return champions.filter((champion) => {
      if (needle.length > 0) {
        const haystack = normalize(`${champion.name} ${champion.title}`);
        if (!haystack.includes(needle)) return false;
      }
      if (roleFilter !== "all") {
        if (!champion.roles.some((fit) => fit.role === roleFilter)) return false;
      }
      if (onlyAvailable && used.has(champion.key)) return false;
      return true;
    });
  }, [champions, query, roleFilter, onlyAvailable, used]);

  const next = draft.nextStep;

  return (
    <section className="panel flex flex-1 flex-col overflow-hidden">
      <div className="flex flex-wrap items-center gap-2 border-b border-line/70 px-3 py-2.5">
        <div className="relative min-w-[180px] flex-1">
          <span
            aria-hidden="true"
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-xs text-muted"
          >
            ⌕
          </span>
          <input
            ref={searchRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Buscar campeón…  /"
            className="field w-full py-1.5 pl-7 text-xs"
            aria-label="Buscar campeón"
          />
        </div>
        <select
          value={roleFilter}
          onChange={(event) =>
            setRoleFilter(event.target.value as Role | "all")
          }
          className="field py-1.5 text-xs"
          aria-label="Filtrar por rol"
        >
          <option value="all">Todos los roles</option>
          {ROLE_ORDER.map((role) => (
            <option key={role} value={role}>
              {ROLE_LABEL[role]}
            </option>
          ))}
        </select>
        <label className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-bg/40 px-2.5 py-1.5 text-[11px] text-muted">
          <input
            type="checkbox"
            checked={onlyAvailable}
            onChange={(event) => setOnlyAvailable(event.target.checked)}
            className="h-3 w-3 accent-[#C89B3C]"
          />
          Solo disponibles
        </label>
        <span className="chip tabular">{filtered.length} campeones</span>
      </div>

      {next && (
        <div className="border-b border-line/70 bg-bg/30 px-3 py-2">
          {selected ? (
            <div className="flex flex-wrap items-center gap-2">
              <ChampionAvatar champion={selected} size="sm" />
              <p className="min-w-0 flex-1 text-xs text-muted">
                <span className="font-semibold text-ink">{selected.name}</span>{" "}
                será{" "}
                <span className="font-semibold text-ink">
                  {KIND_LABEL[next.kind].toLowerCase()}
                </span>{" "}
                de{" "}
                <span
                  className={`font-semibold ${
                    next.side === "blue" ? "text-blue-200" : "text-red-200"
                  }`}
                >
                  {next.side === "blue" ? "Azul" : "Rojo"}
                </span>{" "}
                en la acción {next.number}.
              </p>
              <button
                type="button"
                onClick={onConfirm}
                disabled={busy}
                className={
                  next.side === "blue"
                    ? "btn-blue px-3 py-1.5 text-xs"
                    : "btn-red px-3 py-1.5 text-xs"
                }
              >
                Confirmar <kbd className="text-[10px] opacity-70">↵</kbd>
              </button>
              <button
                type="button"
                onClick={() => onSelect(null)}
                className="btn-ghost px-3 py-1.5 text-xs"
              >
                Cancelar <kbd className="text-[10px] opacity-70">Esc</kbd>
              </button>
            </div>
          ) : (
            <p className="text-[11px] text-muted">
              <span className="font-semibold text-ink">Turno actual:</span>{" "}
              {next.side === "blue" ? "Azul" : "Rojo"}{" "}
              {KIND_LABEL[next.kind].toLowerCase()} · acción {next.number} de{" "}
              {draft.totalActions}. Elige un campeón disponible.
            </p>
          )}
        </div>
      )}

      <div className="max-h-[clamp(220px,38vh,380px)] flex-1 overflow-y-auto p-2">
        {filtered.length === 0 ? (
          <div className="flex h-32 flex-col items-center justify-center text-center">
            <p className="text-sm font-semibold text-ink">Sin resultados</p>
            <p className="mt-1 max-w-sm text-xs text-muted">
              Ajusta la búsqueda o los filtros. Los campeones ya usados siguen
              visibles al desactivar «Solo disponibles».
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(58px,1fr))] gap-1.5">
            {filtered.map((champion) => {
              const usage = used.get(champion.key);
              const isSelected = selected?.key === champion.key;
              return (
                <div
                  key={champion.key}
                  className={`group relative ${
                    isSelected
                      ? "drop-shadow-[0_0_7px_rgba(200,155,60,0.65)]"
                      : ""
                  }`}
                >
                  <button
                    type="button"
                    onClick={() => {
                      if (usage || !next) {
                        onInspect(champion.key);
                      } else {
                        onSelect(isSelected ? null : champion);
                      }
                    }}
                    disabled={busy && !usage}
                    title={
                      usage
                        ? `${champion.name} · ${KIND_LABEL[usage.kind]} de ${
                            usage.side === "blue" ? "Azul" : "Rojo"
                          }`
                        : `${champion.name} · ${champion.title}`
                    }
                    className={`portrait relative block aspect-square w-full overflow-hidden border transition ${
                      isSelected
                        ? "border-gold bg-gold/10"
                        : usage
                          ? "border-line/50 opacity-55 hover:opacity-90"
                          : "border-line hover:-translate-y-0.5 hover:border-gold/60"
                    }`}
                  >
                    <ChampionAvatar
                      champion={champion}
                      size="fill"
                      dimmed={Boolean(usage)}
                      className="rounded-none border-0"
                    />
                    <span className="pointer-events-none absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-bg/95 via-bg/70 to-transparent px-1 pb-0.5 pt-3 text-left text-[9px] font-semibold text-ink">
                      {champion.name}
                    </span>
                    {usage && (
                      <span
                        className={`absolute right-0.5 top-0.5 px-1 text-[8px] font-bold uppercase ${
                          usage.side === "blue"
                            ? "bg-blue text-[#04211F]"
                            : "bg-red text-[#2A0710]"
                        }`}
                      >
                        {usage.kind === "ban" ? "B" : "P"}
                      </span>
                    )}
                  </button>
                  {!usage && (
                    <button
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        onInspect(champion.key);
                      }}
                      className="absolute left-0.5 top-0.5 hidden h-4 w-4 items-center justify-center rounded bg-bg/85 text-[9px] font-bold text-muted hover:text-gold group-hover:flex"
                      aria-label={`Ver detalle de ${champion.name}`}
                      title="Ver detalle"
                    >
                      i
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="flex items-center justify-between border-t border-line/70 px-3 py-1.5 text-[10px] text-muted">
        <span>
          Roles importados:{" "}
          {champions.filter((champion) => champion.roles.length > 0).length} /{" "}
          {champions.length}
        </span>
        <span className="text-muted/70">
          Perfiles estratégicos: {profileSummary}
        </span>
      </div>
    </section>
  );
}
