import { useState } from "react";
import { api } from "../api";
import { counterstatsUrl } from "../lib/format";

/**
 * Abre la página pública de CounterStats del campeón en el navegador.
 *
 * No incrustamos ni copiamos sus estadísticas: solo enlazamos la fuente para
 * que el usuario consulte el enfrentamiento con sus datos actualizados.
 */
export function CounterStatsLink({
  championName,
  label,
  className = "",
}: {
  championName: string;
  label?: string;
  className?: string;
}) {
  const [error, setError] = useState(false);
  const url = counterstatsUrl(championName);

  return (
    <span className="inline-flex flex-col items-start gap-0.5">
      <button
        type="button"
        onClick={() => {
          setError(false);
          void api.openExternal(url).catch(() => setError(true));
        }}
        className={`inline-flex items-center gap-1 text-[10px] font-semibold text-gold/90 underline-offset-2 hover:text-gold hover:underline ${className}`}
        title={`Abrir ${championName} en CounterStats (fuente externa)`}
      >
        <span aria-hidden="true">↗</span>
        {label ?? "CounterStats"}
      </button>
      {error && (
        <span className="text-[9px] text-danger">No se pudo abrir el enlace.</span>
      )}
    </span>
  );
}
