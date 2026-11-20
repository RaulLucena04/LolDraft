import { useCallback, useEffect, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { api } from "../api";
import type { BuildDataset, CommandError } from "../types";
import { formatRelative, STATS_QUEUE_LABEL } from "../lib/format";
import type { Toast } from "./ToastStack";

interface BuildsCardProps {
  onNotify: (kind: Toast["kind"], message: string) => void;
}

/**
 * Importación y gestión de builds y runas observadas.
 *
 * Como las estadísticas, la app no descarga nada: el usuario elige un JSON
 * local. El dataset se activa para su cola y SoloQ/profesional no se mezclan.
 */
export function BuildsCard({ onNotify }: BuildsCardProps) {
  const [datasets, setDatasets] = useState<BuildDataset[]>([]);
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const jsonInput = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    try {
      const next = await api.builds();
      setDatasets(next);
      setError(null);
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      setError(
        commandError.message ??
          "No se pudieron leer los datasets de builds.",
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const handleJsonFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setImporting(true);
    try {
      const text = await file.text();
      const dataset = await api.buildsImportJson(text, true);
      onNotify(
        "success",
        `Builds ${dataset.id} · ${dataset.version} importadas y activas: ${dataset.rowCount} filas · ${STATS_QUEUE_LABEL[dataset.queue]} · parche ${dataset.patch}.`,
      );
      await load();
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ?? "No se pudo importar el documento de builds.",
      );
    } finally {
      setImporting(false);
    }
  };

  const handleActivate = async (id: string) => {
    try {
      setDatasets(await api.buildsActivate(id));
      onNotify("success", `Dataset de builds ${id} activo para su cola.`);
      setError(null);
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ?? "No se pudo activar el dataset de builds.",
      );
    }
  };

  const handleDelete = async (dataset: BuildDataset) => {
    if (
      !window.confirm(
        `¿Eliminar el dataset de builds ${dataset.id}? Sus ${dataset.rowCount} filas se borran de la base local.`,
      )
    ) {
      return;
    }
    try {
      await api.buildsDelete(dataset.id);
      onNotify("info", `Dataset de builds ${dataset.id} eliminado.`);
      await load();
    } catch (caught) {
      const commandError = caught as Partial<CommandError>;
      onNotify(
        "error",
        commandError.message ?? "No se pudo eliminar el dataset de builds.",
      );
    }
  };

  const active = datasets.filter((dataset) => dataset.isActive);

  return (
    <div className="panel p-5">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="eyebrow">Fase 4 · Builds y runas</p>
          <h2 className="mt-2 text-base font-bold text-ink">
            Builds importadas
          </h2>
        </div>
        <span className="chip tabular">{datasets.length} datasets</span>
      </div>

      <p className="mt-2 text-[11px] leading-relaxed text-muted">
        La build se elige con el mejor winrate ajustado del propio dataset y se
        muestra al lado de tu pick con sus objetos, runas, muestra y fuente. Sin
        dataset no se inventa ninguna build.
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
              <span className="tabular text-muted">{dataset.rowCount} builds</span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-3 rounded-lg border border-dashed border-line px-3 py-2.5 text-[11px] text-muted">
          Ningún dataset de builds activo. Los picks se recomiendan igual, pero
          no aparece build ni página de runas.
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
                v{dataset.version} · parche {dataset.patch} · {dataset.rowCount}{" "}
                builds · {formatRelative(dataset.createdAt)}
              </p>
              <p
                className="mt-0.5 truncate text-[10px] text-muted/80"
                title={dataset.source}
              >
                {dataset.source}
              </p>
              <p
                className="mt-0.5 truncate text-[10px] text-muted/70"
                title={dataset.license}
              >
                Licencia: {dataset.license}
              </p>
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={() => void handleActivate(dataset.id)}
                  disabled={importing || dataset.isActive}
                  className="btn-ghost px-2.5 py-1 text-[11px]"
                >
                  {dataset.isActive ? "En uso" : "Activar"}
                </button>
                <button
                  type="button"
                  onClick={() => void handleDelete(dataset)}
                  disabled={importing}
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
            Todavía no hay builds importadas.
          </p>
        )
      )}

      {error && (
        <p className="mt-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[11px] text-danger">
          {error}
        </p>
      )}

      <input
        ref={jsonInput}
        type="file"
        accept="application/json,.json"
        className="hidden"
        onChange={handleJsonFile}
      />
      <button
        type="button"
        onClick={() => jsonInput.current?.click()}
        disabled={importing}
        className="btn-gold mt-3 w-full text-xs"
      >
        {importing ? "Importando JSON…" : "Importar JSON de builds"}
      </button>

      <p className="mt-2 text-center text-[10px] leading-relaxed text-muted/70">
        Cada fila declara campeón, rol, objetos y runas. Se rechaza el documento
        entero si una build no es válida: nunca se completa a medias.
      </p>
    </div>
  );
}
