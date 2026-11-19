import { useCallback, useEffect, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { api } from "../api";
import type {
  CommandError,
  StatsCsvMeta,
  StatsDataset,
  StatsDiscardedRow,
  StatsQueue,
} from "../types";
import { formatRelative, STATS_QUEUE_LABEL } from "../lib/format";
import type { Toast } from "./ToastStack";

interface StatsDatasetsCardProps {
  /** Parche del catálogo, usado como valor por defecto al importar un CSV. */
  defaultPatch: string | null;
  onNotify: (kind: Toast["kind"], message: string) => void;
}

/** Nombre de fichero sin extensión, como id por defecto de un CSV. */
function fileStem(name: string): string {
  const stem = name.replace(/\.[^./\\]+$/, "").trim();
  return stem.length > 0 ? stem : "estadisticas";
}

/**
 * Importación y gestión de estadísticas observadas.
 *
 * La app no descarga nada: el usuario elige un fichero local. El dataset
 * importado se activa para su cola; SoloQ y profesional se guardan aparte.
 */
export function StatsDatasetsCard({
  defaultPatch,
  onNotify,
}: StatsDatasetsCardProps) {
  const [datasets, setDatasets] = useState<StatsDataset[]>([]);
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState<"json" | "csv" | null>(null);
  const [queue, setQueue] = useState<StatsQueue>("soloq");
  const [patch, setPatch] = useState(defaultPatch ?? "");
  const [discarded, setDiscarded] = useState<StatsDiscardedRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const jsonInput = useRef<HTMLInputElement>(null);
  const csvInput = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    try {
      const next = await api.stats();
      setDatasets(next);
      setError(null);
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      setError(
        commandError.message ??
          "No se pudieron leer los datasets de estadísticas.",
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const busy = importing !== null;

  const handleJsonFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setImporting("json");
    setDiscarded([]);
    try {
      const text = await file.text();
      const dataset = await api.statsImportJson(text, true);
      onNotify(
        "success",
        `Estadísticas ${dataset.id} · ${dataset.version} importadas y activas: ${dataset.rowCount} filas · ${STATS_QUEUE_LABEL[dataset.queue]} · parche ${dataset.patch}.`,
      );
      await load();
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ??
          "No se pudo importar el documento de estadísticas.",
      );
    } finally {
      setImporting(null);
    }
  };

  const handleCsvFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setImporting("csv");
    setDiscarded([]);
    const meta: StatsCsvMeta = {
      id: fileStem(file.name),
      version: "1",
      source: "Importación manual",
      license: "Uso personal; no redistribuir",
      queue,
      patch: patch.trim() || defaultPatch || "",
    };
    try {
      const text = await file.text();
      const result = await api.statsImportCsv(text, meta, true);
      setDiscarded(result.discarded);
      onNotify(
        result.discarded.length > 0 ? "info" : "success",
        result.summary,
      );
      await load();
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ?? "No se pudo importar el CSV de estadísticas.",
      );
    } finally {
      setImporting(null);
    }
  };

  const handleActivate = async (id: string) => {
    try {
      setDatasets(await api.statsActivate(id));
      onNotify("success", `Dataset ${id} activo para su cola.`);
      setError(null);
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ?? "No se pudo activar el dataset.",
      );
    }
  };

  const handleDelete = async (dataset: StatsDataset) => {
    if (
      !window.confirm(
        `¿Eliminar el dataset ${dataset.id}? Sus ${dataset.rowCount} filas se borran de la base local.`,
      )
    ) {
      return;
    }
    try {
      await api.statsDelete(dataset.id);
      onNotify("info", `Dataset ${dataset.id} eliminado.`);
      await load();
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ?? "No se pudo eliminar el dataset.",
      );
    }
  };

  const active = datasets.filter((dataset) => dataset.isActive);

  return (
    <div className="panel p-5">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="eyebrow">Fase 4 · Datos observados</p>
          <h2 className="mt-2 text-base font-bold text-ink">
            Estadísticas importadas
          </h2>
        </div>
        <span className="chip tabular">{datasets.length} datasets</span>
      </div>

      <p className="mt-2 text-[11px] leading-relaxed text-muted">
        Solo se leen ficheros que elijas tú. Nada se descarga ni se raspa. El
        winrate se ajusta con shrinkage; por debajo de 200 partidas el dato se
        muestra pero no puntúa.
      </p>

      {active.length > 0 ? (
        <ul className="mt-3 space-y-1">
          {active.map((dataset) => (
            <li
              key={dataset.id}
              className="flex items-center gap-2 rounded-lg border border-success/25 bg-success/5 px-3 py-2 text-[11px]"
            >
              <span className="h-1.5 w-1.5 rounded-full bg-success" />
              <span className="font-semibold text-success">
                {STATS_QUEUE_LABEL[dataset.queue]}
              </span>
              <span className="min-w-0 flex-1 truncate text-ink">
                {dataset.id} · parche {dataset.patch}
              </span>
              <span className="tabular text-muted">{dataset.rowCount} filas</span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-3 rounded-lg border border-dashed border-line px-3 py-2.5 text-[11px] text-muted">
          Ningún dataset activo. Sin estadísticas observadas, las
          recomendaciones siguen usando solo el perfil estratégico.
        </p>
      )}

      {loading ? (
        <p className="mt-4 text-[11px] text-muted">Leyendo datasets…</p>
      ) : datasets.length > 0 ? (
        <ul className="mt-4 divide-y divide-line/70">
          {datasets.map((dataset) => (
            <li key={dataset.id} className="py-3 first:pt-0 last:pb-0">
              <div className="flex items-center gap-2">
                <span className="min-w-0 flex-1 truncate text-xs font-semibold text-ink">
                  {dataset.id}
                </span>
                {dataset.isActive && (
                  <span className="chip border-success/35 text-success">
                    Activo
                  </span>
                )}
                <span className="chip">{STATS_QUEUE_LABEL[dataset.queue]}</span>
              </div>
              <p className="mt-1 text-[10px] text-muted">
                v{dataset.version} · parche {dataset.patch} ·{" "}
                {dataset.rowCount} filas · {formatRelative(dataset.createdAt)}
              </p>
              <p className="mt-0.5 truncate text-[10px] text-muted/80" title={dataset.source}>
                {dataset.source}
              </p>
              <p className="mt-0.5 truncate text-[10px] text-muted/70" title={dataset.license}>
                Licencia: {dataset.license}
              </p>
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={() => void handleActivate(dataset.id)}
                  disabled={busy || dataset.isActive}
                  className="btn-ghost px-2.5 py-1 text-[11px]"
                >
                  {dataset.isActive ? "En uso" : "Activar"}
                </button>
                <button
                  type="button"
                  onClick={() => void handleDelete(dataset)}
                  disabled={busy}
                  className="rounded-md border border-line px-2.5 py-1 text-[11px] text-muted transition hover:border-danger/50 hover:text-danger disabled:opacity-40"
                >
                  Eliminar
                </button>
              </div>
            </li>
          ))}
        </ul>
      ) : (
        !error && (
          <p className="mt-4 rounded-lg border border-dashed border-line px-3 py-2.5 text-[11px] text-muted">
            Todavía no hay estadísticas importadas.
          </p>
        )
      )}

      {error && (
        <p className="mt-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[11px] text-danger">
          {error}
        </p>
      )}

      <div className="mt-4 grid grid-cols-2 gap-2">
        <div>
          <label
            htmlFor="stats-queue"
            className="text-[10px] font-semibold uppercase tracking-wider text-muted"
          >
            Cola del CSV
          </label>
          <select
            id="stats-queue"
            value={queue}
            onChange={(event) => setQueue(event.target.value as StatsQueue)}
            disabled={busy}
            className="field mt-1 w-full py-1.5 text-xs"
          >
            <option value="soloq">SoloQ</option>
            <option value="pro">Profesional</option>
          </select>
        </div>
        <div>
          <label
            htmlFor="stats-patch"
            className="text-[10px] font-semibold uppercase tracking-wider text-muted"
          >
            Parche del CSV
          </label>
          <input
            id="stats-patch"
            value={patch}
            onChange={(event) => setPatch(event.target.value)}
            disabled={busy}
            placeholder="16.22"
            className="field mt-1 w-full py-1.5 text-xs"
          />
        </div>
      </div>
      <p className="mt-1.5 text-[10px] leading-relaxed text-muted/70">
        Un CSV puede declarar sus propios metadatos con líneas{" "}
        <code># id:</code>, <code># parche:</code>, <code># cola:</code>,{" "}
        <code># fuente:</code> o <code># licencia:</code>.
      </p>

      <input
        ref={jsonInput}
        type="file"
        accept="application/json,.json"
        className="hidden"
        onChange={handleJsonFile}
      />
      <input
        ref={csvInput}
        type="file"
        accept="text/csv,.csv"
        className="hidden"
        onChange={handleCsvFile}
      />

      <div className="mt-3 grid gap-2">
        <button
          type="button"
          onClick={() => jsonInput.current?.click()}
          disabled={busy}
          className="btn-gold w-full text-xs"
        >
          {importing === "json" ? "Importando JSON…" : "Importar JSON"}
        </button>
        <button
          type="button"
          onClick={() => csvInput.current?.click()}
          disabled={busy}
          className="btn-ghost w-full text-xs"
        >
          {importing === "csv" ? "Importando CSV…" : "Importar CSV"}
        </button>
      </div>

      <p className="mt-2 text-center text-[10px] leading-relaxed text-muted/70">
        El dataset importado se activa en su cola. Las filas inválidas de un CSV
        se descartan con su motivo; un JSON estricto se rechaza entero.
      </p>

      {discarded.length > 0 && (
        <div className="mt-3 rounded-lg border border-warning/30 bg-warning/5 px-3 py-2.5">
          <p className="text-[11px] font-semibold text-warning">
            {discarded.length} filas en cuarentena
          </p>
          <ul className="mt-1.5 space-y-1">
            {discarded.slice(0, 6).map((row, index) => (
              <li
                key={`${row.line ?? "fila"}-${index}`}
                className="text-[10px] leading-relaxed text-warning/90"
              >
                {row.line !== null ? `Línea ${row.line}: ` : ""}
                {row.reason}
              </li>
            ))}
            {discarded.length > 6 && (
              <li className="text-[10px] text-warning/70">
                y {discarded.length - 6} más…
              </li>
            )}
          </ul>
        </div>
      )}
    </div>
  );
}
