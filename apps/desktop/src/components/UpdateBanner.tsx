import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { CommandError, UpdateCheck } from "../types";

const DISMISSED_KEY = "loldraft.updates.dismissed";

function readDismissed(): string | null {
  try {
    return window.localStorage.getItem(DISMISSED_KEY);
  } catch {
    return null;
  }
}

function writeDismissed(version: string): void {
  try {
    window.localStorage.setItem(DISMISSED_KEY, version);
  } catch {
    // Sin almacenamiento, el aviso simplemente vuelve en el próximo arranque.
  }
}

function messageOf(error: unknown): string {
  const commandError = error as Partial<CommandError>;
  return commandError.message ?? "No se pudo actualizar LolDraft.";
}

/**
 * Aviso de actualización que se comprueba al arrancar.
 *
 * La instalación es sobre la propia app: se descarga el instalador del canal,
 * se verifica su `sha256` y se ejecuta en silencio; la app se reinicia con la
 * versión nueva. Un canal HTTP sin checksum abre la descarga en el navegador.
 */
export function UpdateBanner() {
  const [check, setCheck] = useState<UpdateCheck | null>(null);
  const [hidden, setHidden] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void api
      .updateCheck()
      .then((next) => {
        if (!alive) return;
        setCheck(next);
        if (next.available && readDismissed() === next.latestVersion) {
          setHidden(true);
        }
      })
      .catch(() => {
        // Sin red o sin canal no se molesta al usuario al arrancar.
      });
    return () => {
      alive = false;
    };
  }, []);

  const dismiss = useCallback(() => {
    setHidden(true);
    if (check) writeDismissed(check.latestVersion);
  }, [check]);

  const install = useCallback(async () => {
    if (!check?.available) return;
    setInstalling(true);
    setError(null);
    setNotice(null);
    try {
      const started = await api.updateInstall(check.latestVersion);
      setNotice(started.message);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setInstalling(false);
    }
  }, [check]);

  if (!check?.available || hidden) return null;

  return (
    <div className="pointer-events-auto fixed bottom-5 right-5 z-40 w-[380px] max-w-[calc(100vw-2rem)] animate-rise">
      <div className="panel border-gold/45 bg-[#1a1708] p-4 shadow-panel">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="eyebrow text-gold">Actualización disponible</p>
            <p className="mt-1 text-sm font-bold text-ink">
              LolDraft {check.latestVersion}
            </p>
          </div>
          <button
            type="button"
            onClick={dismiss}
            className="rounded px-1 text-muted hover:text-ink"
            aria-label="Recordar más tarde"
            title="Recordar más tarde"
          >
            ×
          </button>
        </div>

        {check.notes && (
          <p className="mt-1.5 text-[11px] leading-relaxed text-muted">
            {check.notes}
          </p>
        )}

        {notice ? (
          <p className="mt-2 rounded-lg border border-success/40 bg-success/10 px-3 py-2 text-[11px] leading-relaxed text-success">
            {notice}
          </p>
        ) : (
          <>
            <p className="mt-2 text-[11px] leading-relaxed text-muted">
              Se descarga del canal, se verifica su <code>sha256</code> y se
              instala sobre esta app. LolDraft se cerrará y volverá a abrir con
              la versión nueva.
            </p>
            {error && (
              <p className="mt-2 rounded-lg border border-danger/35 bg-danger/5 px-3 py-2 text-[11px] leading-relaxed text-danger">
                {error}
              </p>
            )}
            <button
              type="button"
              onClick={() => void install()}
              disabled={installing}
              className="btn-gold mt-2.5 w-full text-xs"
            >
              {installing ? "Descargando e instalando…" : "Actualizar ahora"}
            </button>
            <button
              type="button"
              onClick={dismiss}
              disabled={installing}
              className="mt-1.5 w-full text-[10px] text-muted hover:text-ink"
            >
              Recordar más tarde
            </button>
          </>
        )}
      </div>
    </div>
  );
}
