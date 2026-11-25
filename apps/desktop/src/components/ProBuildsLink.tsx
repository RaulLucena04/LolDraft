import { useState } from "react";
import { api } from "../api";
import { probuildsUrl } from "../lib/format";

/**
 * Abre la página pública de ProBuilds del campeón en el navegador.
 *
 * No incrustamos ni copiamos sus builds: solo enlazamos la fuente para que el
 * usuario compare con partidas reales de profesionales.
 */
export function ProBuildsLink({
  championSlug,
  championName,
  className = "",
}: {
  championSlug: string;
  championName: string;
  className?: string;
}) {
  const [error, setError] = useState(false);

  return (
    <span className="inline-flex flex-col items-start gap-0.5">
      <button
        type="button"
        onClick={() => {
          setError(false);
          void api
            .openExternal(probuildsUrl(championSlug))
            .catch(() => setError(true));
        }}
        className={`inline-flex items-center gap-1 text-[10px] font-semibold text-gold/90 underline-offset-2 hover:text-gold hover:underline ${className}`}
        title={`Abrir las builds de ${championName} en ProBuilds (fuente externa)`}
      >
        <span aria-hidden="true">↗</span>
        Builds de pros
      </button>
      {error && (
        <span className="text-[9px] text-danger">No se pudo abrir el enlace.</span>
      )}
    </span>
  );
}
