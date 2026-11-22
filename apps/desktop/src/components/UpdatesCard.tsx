import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { CommandError, UpdateCheck } from "../types";

interface UpdatesCardProps {
  currentVersion: string;
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
 * Tarjeta de actualizaciones: consulta el canal configurado y actualiza sobre
 * la propia instalación. Al arrancar, `UpdateBanner` ya hace la comprobación;
 * esta tarjeta permite revisar el canal y lanzar la actualización a mano.
 */
export function UpdatesCard({ currentVersion }: UpdatesCardProps) {
  const [source, setSource] = useState("");
  const [savedSource, setSavedSource] = useState("");
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [result, setResult] = useState<UpdateCheck | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void (async () => {
      try {
        const saved = await api.settingGet("updates.source");
        if (!alive || typeof saved !== "string") return;
        setSource(saved);
        setSavedSource(saved);
      } catch {
        // Un ajuste ilegible no impide comprobar el canal por defecto.
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  const check = useCallback(async () => {
    setChecking(true);
    setError(null);
    setNotice(null);
    setResult(null);
    const trimmed = source.trim();
    try {
      if (trimmed !== savedSource) {
        await api.settingUpdate("updates.source", trimmed);
        setSavedSource(trimmed);
      }
      setResult(await api.updateCheck());
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setChecking(false);
    }
  }, [source, savedSource]);

  const install = useCallback(async () => {
    if (!result?.available) return;
    if (
      !window.confirm(
        `Se descargará LolDraft ${result.latestVersion}, se verificará y se instalará sobre esta app. LolDraft se cerrará y volverá a abrir. ¿Continuar?`,
      )
    ) {
      return;
    }
    setInstalling(true);
    setError(null);
    setNotice(null);
    try {
      const installed = await api.updateInstall(result.latestVersion);
      setNotice(installed.message);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setInstalling(false);
    }
  }, [result]);

  return (
    <div className="panel p-5">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="eyebrow">Aplicación</p>
          <h2 className="mt-2 text-base font-bold text-ink">
            Versión {currentVersion}
          </h2>
        </div>
        <span
          className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${
            result?.available
              ? "border-gold/60 bg-gold/10 text-gold"
              : "border-line bg-bg/40 text-muted"
          }`}
        >
          {result?.available ? "Actualización" : "Instalada"}
        </span>
      </div>

      <p className="mt-2 text-xs leading-relaxed text-muted">
        Al arrancar se comprueba el canal público de LolDraft o la carpeta o URL
        que indiques. El canal es un <code className="text-ink">latest.json</code>{" "}
        junto a su instalador.
      </p>

      <label
        htmlFor="updates-source"
        className="mt-3 block text-[10px] font-semibold uppercase tracking-wider text-muted"
      >
        Origen (vacío = releases de GitHub)
      </label>
      <input
        id="updates-source"
        type="text"
        value={source}
        onChange={(event) => setSource(event.target.value)}
        spellCheck={false}
        autoComplete="off"
        placeholder={"C:\\ruta\\a\\updates o https://…/updates"}
        className="field mt-1 w-full text-xs"
      />

      <button
        type="button"
        onClick={() => void check()}
        disabled={checking || installing}
        className="btn-gold mt-2 w-full text-xs"
      >
        {checking ? "Buscando…" : "Buscar actualizaciones"}
      </button>

      {error && (
        <p className="mt-3 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-[11px] leading-relaxed text-warning">
          {error}
        </p>
      )}

      {notice && (
        <p className="mt-3 rounded-lg border border-success/40 bg-success/10 px-3 py-2 text-[11px] leading-relaxed text-success">
          {notice}
        </p>
      )}

      {result && (
        <div className="mt-3 rounded-lg border border-line/80 bg-bg/40 px-3 py-2">
          {result.available ? (
            <>
              <p className="text-xs font-semibold text-ink">
                Hay una versión más nueva: {result.latestVersion}
              </p>
              {result.notes && (
                <p className="mt-1 text-[11px] leading-relaxed text-muted">
                  {result.notes}
                </p>
              )}
              <p className="mt-1 break-all text-[10px] leading-relaxed text-muted/80">
                {result.installer}
              </p>
              <button
                type="button"
                onClick={() => void install()}
                disabled={installing}
                className="btn-gold mt-2 w-full text-xs"
              >
                {installing
                  ? "Descargando e instalando…"
                  : `Actualizar a ${result.latestVersion}`}
              </button>
            </>
          ) : (
            <p className="text-xs leading-relaxed text-muted">
              Estás al día. El canal publica la versión{" "}
              <span className="font-semibold text-ink">
                {result.latestVersion}
              </span>
              .
            </p>
          )}
          <p className="mt-1.5 break-all text-[10px] leading-relaxed text-muted/70">
            Canal {result.channel === "url" ? "HTTP" : "local"}:{" "}
            {result.manifest}
          </p>
        </div>
      )}

      <p className="mt-2 text-[10px] leading-relaxed text-muted/70">
        La actualización se descarga del canal y se verifica con su{" "}
        <code>sha256</code> antes de ejecutarse. Si el origen es HTTP abierto o
        no publica checksum, la app no ejecuta nada: abre la descarga en el
        navegador. Sin firma Authenticode: Windows puede mostrar SmartScreen.
      </p>
    </div>
  );
}
