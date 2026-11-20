import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  Champion,
  CommandError,
  RecommendationSet,
  Role,
  Side,
  SoloqDraftInput,
  SoloqImport,
} from "../types";
import { api } from "../api";
import { ROLE_LABEL, ROLE_ORDER, SIDE_LABEL } from "../lib/format";
import { RoleBadge, SideBadge } from "./Badges";
import { ChampionAvatar } from "./ChampionAvatar";
import { ChampionBuildPanel } from "./ChampionBuildPanel";
import { RecommendationsPanel } from "./RecommendationsPanel";

interface SoloqScreenProps {
  champions: Champion[];
  hasProfiles: boolean;
  onHome: () => void;
  onInspect: (championKey: number) => void;
}

type SlotTeam = "ally" | "enemy" | "ban";

interface EditingSlot {
  team: SlotTeam;
  index: number;
}

/** Estado de la sincronización con el cliente, sin detalles que cambien en cada tick. */
type SyncState =
  | { kind: "syncing" }
  | { kind: "live"; phase: string | null }
  | { kind: "closed" }
  | { kind: "idle" }
  | { kind: "off" }
  | { kind: "error"; message: string };

const SYNC_INTERVAL_MS = 2000;

const emptyAllies = (): (number | null)[] => [null, null, null, null];
const emptyEnemies = (): (number | null)[] => [null, null, null, null, null];
const emptyBans = (): (number | null)[] =>
  Array.from({ length: 10 }, () => null);

function normalize(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function filled(values: (number | null)[]): number[] {
  return values.filter((key): key is number => key !== null);
}

function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as CommandError).message === "string"
  ) {
    return (error as CommandError).message;
  }
  return String(error);
}

/**
 * Traduce un fallo del cliente a un estado de sincronización. Estar fuera de
 * champ select o con el cliente cerrado es normal, no un error rojo.
 */
function syncStateFromError(error: unknown): SyncState {
  const code =
    typeof error === "object" && error !== null && "code" in error
      ? String((error as CommandError).code)
      : "";
  switch (code) {
    case "lcu_not_running":
      return { kind: "closed" };
    case "lcu_not_in_champ_select":
      return { kind: "idle" };
    case "lcu_unreachable":
      return {
        kind: "error",
        message: "No se pudo hablar con el cliente de League of Legends.",
      };
    default:
      return { kind: "error", message: errorMessage(error) };
  }
}

export function SoloqScreen({
  champions,
  hasProfiles,
  onHome,
  onInspect,
}: SoloqScreenProps) {
  const [side, setSide] = useState<Side>("blue");
  const [slot, setSlotNumber] = useState(1);
  const [role, setRole] = useState<Role | "">("");
  const [allies, setAllies] = useState<(number | null)[]>(emptyAllies);
  const [enemies, setEnemies] = useState<(number | null)[]>(emptyEnemies);
  const [bans, setBans] = useState<(number | null)[]>(emptyBans);
  const [ownPick, setOwnPick] = useState<number | null>(null);
  const [editing, setEditing] = useState<EditingSlot | null>(null);
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<RecommendationSet | null>(null);
  const [recommending, setRecommending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [autoSync, setAutoSync] = useState(true);
  const [sync, setSync] = useState<SyncState>({ kind: "syncing" });
  const [refreshing, setRefreshing] = useState(false);
  const lastPayload = useRef("");

  const byKey = useMemo(() => {
    const map = new Map<number, Champion>();
    for (const champion of champions) map.set(champion.key, champion);
    return map;
  }, [champions]);

  const usedKeys = useMemo(() => {
    const keys = new Set<number>();
    for (const key of [...filled(allies), ...filled(enemies), ...filled(bans)]) {
      keys.add(key);
    }
    if (ownPick !== null) keys.add(ownPick);
    return keys;
  }, [allies, enemies, bans, ownPick]);

  // Cualquier cambio en el estado del draft invalida el resultado anterior:
  // no se muestra una recomendación calculada para otro estado.
  useEffect(() => {
    setResult(null);
  }, [side, slot, role, allies, enemies, bans, ownPick]);

  const setSlotChampion = (target: EditingSlot, key: number | null) => {
    const update = (values: (number | null)[]) =>
      values.map((value, index) => (index === target.index ? key : value));
    if (target.team === "ally") setAllies(update);
    else if (target.team === "enemy") setEnemies(update);
    else setBans(update);
  };

  const clearAll = () => {
    setAllies(emptyAllies());
    setEnemies(emptyEnemies());
    setBans(emptyBans());
    setOwnPick(null);
    // El cliente sigue mandando: se vuelve a leer en el siguiente tick.
    lastPayload.current = "";
    setResult(null);
    setError(null);
    setNotice(null);
    setEditing(null);
    setQuery("");
  };

  /** Vuelca una lectura del cliente sin inventar huecos ni campeones desconocidos. */
  const applyImport = useCallback((imported: SoloqImport, announce: boolean) => {
    const unknown = new Set(imported.unknownChampions);
    const usable = (key: number) => !unknown.has(key);
    const fill = (keys: number[], size: number): (number | null)[] => {
      const next: (number | null)[] = Array.from({ length: size }, () => null);
      keys
        .filter(usable)
        .slice(0, size)
        .forEach((key, index) => {
          next[index] = key;
        });
      return next;
    };

    setSide(imported.side);
    setSlotNumber(Math.min(5, Math.max(1, imported.slot)));
    setRole(imported.role ?? "");
    setOwnPick(
      imported.ownPick !== null && usable(imported.ownPick)
        ? imported.ownPick
        : null,
    );
    const nextAllies = fill(imported.allyPicks, 4);
    const nextEnemies = fill(imported.enemyPicks, 5);
    const nextBans = fill(imported.bans, 10);
    setAllies(nextAllies);
    setEnemies(nextEnemies);
    setBans(nextBans);
    setResult(null);

    if (announce) {
      setNotice(
        imported.unknownChampions.length > 0
          ? `Se leyeron ${nextAllies.filter((k) => k !== null).length + nextEnemies.filter((k) => k !== null).length} picks y ${nextBans.filter((k) => k !== null).length} baneos. ${imported.unknownChampions.length} campeón(es) del cliente no están en tu catálogo y se omitieron.`
          : "Draft sincronizado con el cliente. Pulsa «Ver mejor pick» cuando quieras el análisis.",
      );
    }
  }, []);

  /** Una lectura del cliente: solo aplica el estado si ha cambiado. */
  const readClient = useCallback(
    async (announce: boolean) => {
      try {
        const imported = await api.lcuChampSelect();
        const key = JSON.stringify(imported);
        if (key !== lastPayload.current) {
          lastPayload.current = key;
          applyImport(imported, announce);
        }
        setSync((previous) =>
          previous.kind === "live" && previous.phase === imported.timerPhase
            ? previous
            : { kind: "live", phase: imported.timerPhase },
        );
      } catch (caught) {
        lastPayload.current = "";
        const next = syncStateFromError(caught);
        setSync((previous) =>
          JSON.stringify(previous) === JSON.stringify(next) ? previous : next,
        );
      }
    },
    [applyImport],
  );

  // Sincronización continua: mientras la pantalla está abierta se relee el
  // cliente cada dos segundos. Los fallos (cerrado, sin champ select) solo
  // cambian el estado visible; nunca borran el draft ya leído.
  useEffect(() => {
    if (!autoSync) {
      setSync({ kind: "off" });
      return;
    }
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      await readClient(false);
      if (!cancelled) timer = window.setTimeout(tick, SYNC_INTERVAL_MS);
    };
    void tick();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [autoSync, readClient]);

  const refreshNow = async () => {
    setRefreshing(true);
    setError(null);
    try {
      await readClient(true);
    } finally {
      setRefreshing(false);
    }
  };

  const handleRecommend = async () => {
    setRecommending(true);
    setError(null);
    const payload: SoloqDraftInput = {
      side,
      slot,
      role: role === "" ? null : role,
      bans: filled(bans),
      allyPicks: filled(allies),
      enemyPicks: filled(enemies),
    };
    try {
      const output = await api.soloqRecommend(payload);
      setResult(output);
    } catch (recommendError) {
      setResult(null);
      setError(errorMessage(recommendError));
    } finally {
      setRecommending(false);
    }
  };

  const best = result?.picks[0] ?? null;

  const available = useMemo(() => {
    const needle = normalize(query.trim());
    return champions.filter((champion) => {
      if (usedKeys.has(champion.key)) return false;
      if (needle.length === 0) return true;
      return normalize(`${champion.name} ${champion.title}`).includes(needle);
    });
  }, [champions, query, usedKeys]);

  const renderSlot = (
    team: SlotTeam,
    index: number,
    label: string,
    key: number | null,
  ) => {
    const champion = key === null ? null : byKey.get(key) ?? null;
    const active =
      editing !== null && editing.team === team && editing.index === index;

    return (
      <div
        key={`${team}-${index}`}
        className={`relative rounded-lg border p-1.5 ${
          active ? "border-gold/70 bg-gold/5" : "border-line bg-surface2/40"
        }`}
      >
        <button
          type="button"
          onClick={() => {
            setEditing({ team, index });
            setQuery("");
          }}
          className="block w-full text-left"
          title={label}
        >
          <span className="flex items-center gap-1.5">
            {champion ? (
              <ChampionAvatar champion={champion} size="sm" />
            ) : (
              <span
                aria-hidden="true"
                className="flex h-8 w-8 items-center justify-center rounded-md border border-dashed border-line text-xs text-muted"
              >
                +
              </span>
            )}
            <span className="min-w-0">
              <span className="block truncate text-[11px] font-semibold text-ink">
                {champion?.name ?? "Vacío"}
              </span>
              <span className="block truncate text-[9px] uppercase tracking-wider text-muted">
                {label}
              </span>
            </span>
          </span>
        </button>
        {champion && (
          <button
            type="button"
            onClick={() => setSlotChampion({ team, index }, null)}
            className="absolute right-1 top-1 flex h-4 w-4 items-center justify-center rounded bg-bg/85 text-[9px] font-bold text-muted hover:text-red-200"
            aria-label={`Vaciar ${label}`}
            title="Vaciar"
          >
            ×
          </button>
        )}
      </div>
    );
  };

  const ownChampion = ownPick === null ? null : (byKey.get(ownPick) ?? null);

  // Build del pick propio si ya está bloqueado; si no, la del mejor pick.
  const buildTarget = ownChampion ?? best?.champion ?? null;
  const buildRole = ownChampion
    ? role || best?.role || null
    : best?.role ?? null;

  const myPickCard = (
    <div
      key="my-pick"
      className="rounded-lg border border-gold/30 bg-gold/5 p-1.5"
      title={
        ownChampion
          ? "Ya has bloqueado este campeón en el cliente"
          : "Tu pick es el que se va a recomendar"
      }
    >
      <span className="flex items-center gap-1.5">
        {ownChampion ? (
          <ChampionAvatar champion={ownChampion} size="sm" />
        ) : (
          <span
            aria-hidden="true"
            className="flex h-8 w-8 items-center justify-center rounded-md border border-dashed border-gold/40 text-[10px] font-bold text-gold"
          >
            Tú
          </span>
        )}
        <span className="min-w-0">
          <span className="block truncate text-[11px] font-semibold text-ink">
            {ownChampion?.name ?? "Tu pick"}
          </span>
          <span className="block truncate text-[9px] uppercase tracking-wider text-muted">
            {ownChampion ? `Bloqueado · turno ${slot}` : `Turno ${slot}`}
          </span>
        </span>
      </span>
    </div>
  );

  const editingLabel = editing
    ? editing.team === "ban"
      ? `baneo ${editing.index + 1}`
      : `${editing.team === "ally" ? "tu equipo" : "rival"} ${
          editing.index + 1
        }`
    : "";

  return (
    <div className="mx-auto flex min-h-screen w-full max-w-[1400px] flex-col px-6 pb-6 pt-5">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-line/70 pb-4">
        <div className="flex items-center gap-3">
          <button type="button" onClick={onHome} className="btn-ghost px-3 py-1.5 text-xs">
            ← Inicio
          </button>
          <div>
            <p className="eyebrow text-gold">SoloQ</p>
            <h1 className="text-xl font-extrabold text-ink">
              Draft en vivo · mejor pick
            </h1>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => setAutoSync((value) => !value)}
            aria-pressed={autoSync}
            className={`px-3 py-1.5 text-xs ${autoSync ? "btn-ghost border-success/40 text-success" : "btn-ghost"}`}
            title="Lee la selección de campeones del cliente cada dos segundos (solo lectura)"
          >
            {autoSync ? "● Sincronizando" : "○ Sincronización pausada"}
          </button>
          <button
            type="button"
            onClick={() => void refreshNow()}
            disabled={refreshing}
            className="btn-ghost px-3 py-1.5 text-xs"
            title="Leer el cliente ahora mismo"
          >
            {refreshing ? "Leyendo…" : "Actualizar"}
          </button>
          <button type="button" onClick={clearAll} className="btn-ghost px-3 py-1.5 text-xs">
            Limpiar
          </button>
        </div>
      </header>

      <p
        className={`mt-2 flex items-center gap-1.5 text-[11px] ${
          sync.kind === "live"
            ? "text-success"
            : sync.kind === "error"
              ? "text-warning"
              : "text-muted"
        }`}
      >
        <span
          aria-hidden="true"
          className={`h-1.5 w-1.5 rounded-full ${
            sync.kind === "live"
              ? "bg-success"
              : sync.kind === "error"
                ? "bg-warning"
                : "bg-muted/60"
          }`}
        />
        {sync.kind === "live"
          ? `Cliente conectado${sync.phase ? ` · fase ${sync.phase}` : ""}. El draft se actualiza solo.`
          : sync.kind === "closed"
            ? "Cliente cerrado: rellena el draft a mano o abre League of Legends."
            : sync.kind === "idle"
              ? "Cliente abierto, fuera de champ select. Esperando a que empiece el draft…"
              : sync.kind === "syncing"
                ? "Buscando al cliente…"
                : sync.kind === "off"
                  ? "Sincronización pausada: el draft no se actualiza solo."
                  : sync.message}
      </p>

      {notice && (
        <p className="mt-3 rounded-md border border-line bg-surface2/60 px-3 py-2 text-[11px] text-muted">
          {notice}
        </p>
      )}

      <main className="mt-4 grid flex-1 grid-cols-1 gap-4 xl:grid-cols-12">
        <div className="flex flex-col gap-4 xl:col-span-7">
          <section className="panel p-4">
            <p className="eyebrow text-gold">Tu situación</p>
            <div className="mt-3 grid gap-3 sm:grid-cols-3">
              <div>
                <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                  Lado
                </p>
                <div className="mt-1 flex gap-1.5">
                  {(["blue", "red"] as Side[]).map((value) => (
                    <button
                      key={value}
                      type="button"
                      onClick={() => setSide(value)}
                      className={`flex-1 rounded-lg border px-2 py-1.5 text-xs font-semibold transition ${
                        side === value
                          ? value === "blue"
                            ? "border-blue bg-blueSoft/50 text-blue-100"
                            : "border-red bg-red/20 text-red-200"
                          : "border-line bg-bg/40 text-muted hover:border-gold/40"
                      }`}
                    >
                      {SIDE_LABEL[value]}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <label
                  htmlFor="soloq-slot"
                  className="text-[10px] font-semibold uppercase tracking-wider text-muted"
                >
                  Tu turno de pick
                </label>
                <select
                  id="soloq-slot"
                  value={slot}
                  onChange={(event) => setSlotNumber(Number(event.target.value))}
                  className="field mt-1 w-full py-1.5 text-xs"
                >
                  {[1, 2, 3, 4, 5].map((value) => (
                    <option key={value} value={value}>
                      Pick {value} de tu equipo
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label
                  htmlFor="soloq-role"
                  className="text-[10px] font-semibold uppercase tracking-wider text-muted"
                >
                  Tu rol (opcional)
                </label>
                <select
                  id="soloq-role"
                  value={role}
                  onChange={(event) => setRole(event.target.value as Role | "")}
                  className="field mt-1 w-full py-1.5 text-xs"
                >
                  <option value="">Sin rol fijo</option>
                  {ROLE_ORDER.map((value) => (
                    <option key={value} value={value}>
                      {ROLE_LABEL[value]}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <p className="mt-2 text-[10px] leading-relaxed text-muted/80">
              Con rol fijo solo se proponen campeones viables en esa posición.
              Sin rol, el motor elige entre los roles libres de tu equipo.
            </p>
          </section>

          <section className="panel p-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="eyebrow text-gold">Draft</p>
              <div className="flex items-center gap-1.5">
                <SideBadge side={side} />
                <span className="text-[10px] uppercase tracking-wider text-muted">
                  Tú eres {SIDE_LABEL[side]}
                </span>
              </div>
            </div>

            <div className="mt-3 space-y-3">
              <div>
                <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                  Tu equipo (picks ya bloqueados)
                </p>
                <div className="mt-1.5 grid grid-cols-2 gap-1.5 sm:grid-cols-5">
                  {[0, 1, 2, 3, 4].map((position) => {
                    if (position === slot - 1) return myPickCard;
                    const allyIndex = position < slot - 1 ? position : position - 1;
                    return renderSlot(
                      "ally",
                      allyIndex,
                      `Aliado ${allyIndex + 1}`,
                      allies[allyIndex] ?? null,
                    );
                  })}
                </div>
              </div>
              <div>
                <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                  Rival
                </p>
                <div className="mt-1.5 grid grid-cols-2 gap-1.5 sm:grid-cols-5">
                  {[0, 1, 2, 3, 4].map((index) =>
                    renderSlot("enemy", index, `Rival ${index + 1}`, enemies[index] ?? null),
                  )}
                </div>
              </div>
              <div>
                <p className="text-[10px] font-semibold uppercase tracking-wider text-muted">
                  Baneos (importante)
                </p>
                <div className="mt-1.5 grid grid-cols-2 gap-1.5 sm:grid-cols-5">
                  {Array.from({ length: 10 }, (_, index) =>
                    renderSlot("ban", index, `Baneo ${index + 1}`, bans[index] ?? null),
                  )}
                </div>
                <p className="mt-1.5 text-[10px] leading-relaxed text-warning/90">
                  Los baneos se leen solos del cliente. Si alguno falta, un campeón
                  ya baneado podría aparecer como recomendación: revísalos antes de
                  pedir el pick.
                </p>
              </div>
            </div>

            {editing && (
              <div className="mt-3 rounded-lg border border-gold/30 bg-bg/40 p-2">
                <div className="flex items-center justify-between gap-2">
                  <p className="text-[10px] font-semibold uppercase tracking-wider text-gold">
                    Elegir campeón para {editingLabel}
                  </p>
                  <button
                    type="button"
                    onClick={() => setEditing(null)}
                    className="text-[10px] font-semibold text-muted hover:text-ink"
                  >
                    Cerrar
                  </button>
                </div>
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Buscar campeón…"
                  className="field mt-2 w-full py-1.5 text-xs"
                  aria-label="Buscar campeón"
                  autoFocus
                />
                <div className="mt-2 max-h-56 overflow-y-auto">
                  {available.length === 0 ? (
                    <p className="px-1 py-3 text-[11px] text-muted">
                      Sin campeones disponibles con esa búsqueda.
                    </p>
                  ) : (
                    <div className="grid grid-cols-[repeat(auto-fill,minmax(52px,1fr))] gap-1">
                      {available.map((champion) => (
                        <button
                          key={champion.key}
                          type="button"
                          onClick={() => {
                            setSlotChampion(
                              { team: editing.team, index: editing.index },
                              champion.key,
                            );
                            setEditing(null);
                            setQuery("");
                          }}
                          title={`${champion.name} · ${champion.title}`}
                          className="relative block aspect-square w-full overflow-hidden rounded-md border border-line transition hover:-translate-y-0.5 hover:border-gold/60"
                        >
                          <ChampionAvatar
                            champion={champion}
                            size="fill"
                            className="rounded-none border-0"
                          />
                          <span className="pointer-events-none absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-bg/95 via-bg/70 to-transparent px-1 pb-0.5 pt-3 text-left text-[9px] font-semibold text-ink">
                            {champion.name}
                          </span>
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}

            <div className="mt-3 flex flex-wrap items-center gap-2">
              <button
                type="button"
                onClick={() => void handleRecommend()}
                disabled={recommending || !hasProfiles || ownChampion !== null}
                className="btn-gold px-4 py-2 text-xs"
              >
                {ownChampion
                  ? "Pick bloqueado"
                  : recommending
                    ? "Calculando…"
                    : "Ver mejor pick"}
              </button>
              {ownChampion && (
                <span className="text-[11px] text-muted">
                  Ya has bloqueado a{" "}
                  <span className="font-semibold text-ink">{ownChampion.name}</span>{" "}
                  en el cliente. No hay pick que recomendar en este draft.
                </span>
              )}
              {!ownChampion && best && (
                <span className="text-[11px] text-muted">
                  Mejor ajuste:{" "}
                  <span className="font-semibold text-ink">
                    {best.champion.name}
                  </span>{" "}
                  ({best.score}/100)
                </span>
              )}
            </div>
          </section>
        </div>

        <div className="flex flex-col gap-4 xl:col-span-5">
          {best && (
            <section className="panel border-gold/30 p-3">
              <p className="eyebrow text-gold">Mejor pick</p>
              <div className="mt-2 flex items-center gap-2.5">
                <ChampionAvatar champion={best.champion} size="md" />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-base font-extrabold text-ink">
                    {best.champion.name}
                  </p>
                  <div className="mt-1 flex flex-wrap items-center gap-1">
                    <RoleBadge role={best.role} />
                    <span className="chip tabular">{best.score}/100 ajuste</span>
                    {best.limitedData && (
                      <span className="chip border-warning/35 text-warning">
                        Datos limitados
                      </span>
                    )}
                  </div>
                </div>
              </div>
              <button
                type="button"
                onClick={() => onInspect(best.champion.key)}
                className="mt-2 text-[10px] font-semibold text-gold/90 hover:text-gold"
              >
                Ver perfil del campeón
              </button>
            </section>
          )}

          {buildTarget && (
            <ChampionBuildPanel
              champion={buildTarget}
              role={buildRole}
              queue="soloq"
              context={ownChampion ? "own" : "suggested"}
            />
          )}

          <RecommendationsPanel
            recommendations={result}
            recommending={recommending}
            error={error}
            hasProfiles={hasProfiles}
            draftComplete={false}
            onInspect={onInspect}
            actionLabel={`Pick ${slot} de tu equipo`}
            emptyHint={
              ownChampion
                ? "Tu pick ya está bloqueado; este panel servirá en el siguiente draft."
                : sync.kind === "live"
                  ? "El draft se rellena solo desde el cliente. Pulsa «Ver mejor pick» cuando quieras el análisis."
                  : "Rellena los picks ya bloqueados y pulsa «Ver mejor pick»."
            }
          />
        </div>
      </main>
    </div>
  );
}
