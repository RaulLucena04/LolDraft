import { useRef } from "react";
import type { ChangeEvent } from "react";
import type { Bootstrap, Side } from "../types";
import { formatRelative } from "../lib/format";
import { Logo } from "./Logo";
import { SideBadge, StatusPill } from "./Badges";
import { StatsDatasetsCard } from "./StatsDatasetsCard";
import type { Toast } from "./ToastStack";
import { UpdatesCard } from "./UpdatesCard";

interface HomeProps {
  bootstrap: Bootstrap;
  busy: boolean;
  syncing: boolean;
  importingProfiles: boolean;
  importingSet: boolean;
  onNewDraft: (side: Side) => void;
  onSoloq: () => void;
  onOpenSession: (id: string) => void;
  onDeleteSession: (id: string) => void;
  onSync: () => void;
  onImportPilot: () => void;
  onImportSet: (file: File) => void;
  onNotify: (kind: Toast["kind"], message: string) => void;
}

export function Home({
  bootstrap,
  busy,
  syncing,
  importingProfiles,
  importingSet,
  onNewDraft,
  onSoloq,
  onOpenSession,
  onDeleteSession,
  onSync,
  onImportPilot,
  onImportSet,
  onNotify,
}: HomeProps) {
  const { data, sessions } = bootstrap;
  const setFileInput = useRef<HTMLInputElement>(null);
  const coverage =
    data.championCount > 0
      ? Math.round((data.championsWithRoles / data.championCount) * 100)
      : 0;

  const handleSetFile = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (file) onImportSet(file);
  };

  return (
    <div className="mx-auto flex min-h-screen w-full max-w-[1400px] flex-col px-6 pb-6 pt-5">
      <header className="flex items-center justify-between gap-4 border-b border-line/70 pb-5">
        <Logo />
        <div className="flex items-center gap-2">
          {data.patch ? (
            <StatusPill tone="info">Parche {data.patch}</StatusPill>
          ) : (
            <StatusPill tone="warn">Parche sin fijar</StatusPill>
          )}
          <StatusPill tone={data.warnings.length ? "warn" : "ok"}>
            {data.warnings.length
              ? `${data.warnings.length} aviso${data.warnings.length === 1 ? "" : "s"} de datos`
              : "Datos locales listos"}
          </StatusPill>
        </div>
      </header>

      <main className="grid flex-1 grid-cols-1 gap-5 py-5 xl:grid-cols-12">
        <section className="flex flex-col gap-5 xl:col-span-8">
          <div className="panel relative overflow-hidden p-7">
            <div
              aria-hidden="true"
              className="pointer-events-none absolute -right-24 -top-28 h-72 w-72 rounded-full bg-blue/10 blur-2xl"
            />
            <div
              aria-hidden="true"
              className="pointer-events-none absolute -bottom-32 right-24 h-64 w-64 rounded-full bg-red/10 blur-2xl"
            />
            <div className="relative">
              <p className="eyebrow text-gold">Simulador · Análisis · SoloQ</p>
              <h1 className="mt-4 max-w-2xl text-4xl font-extrabold leading-[1.08] tracking-tight text-ink">
                Simula el draft.
                <br />
                <span className="text-muted">Entiende la composición.</span>
              </h1>
              <p className="mt-4 max-w-xl text-sm leading-relaxed text-muted">
                Un asistente competitivo en Rust + Tauri. Simulador de las 20
                acciones, análisis de composición y recomendaciones explicables.
                En SoloQ, el draft se sincroniza solo con el cliente y los
                objetos y runas se recomiendan por función frente al rival, sin
                importar JSON.
              </p>

              <div className="mt-7 grid gap-3 sm:grid-cols-2">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => onNewDraft("blue")}
                  className="group relative overflow-hidden rounded-xl border border-blue/50 bg-blueSoft/35 px-5 py-4 text-left transition hover:border-blue hover:bg-blueSoft/55 disabled:opacity-50"
                >
                  <span className="flex items-center gap-2 text-sm font-bold text-blue-100">
                    <span className="h-2 w-2 rounded-full bg-blue" />
                    Empezar con Azul
                  </span>
                  <span className="mt-1 block text-xs text-muted">
                    Tu lado para el análisis posterior
                  </span>
                  <span className="absolute right-4 top-1/2 -translate-y-1/2 text-xl text-blue/70 transition group-hover:translate-x-0.5">
                    →
                  </span>
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => onNewDraft("red")}
                  className="group relative overflow-hidden rounded-xl border border-red/50 bg-redSoft/35 px-5 py-4 text-left transition hover:border-red hover:bg-redSoft/55 disabled:opacity-50"
                >
                  <span className="flex items-center gap-2 text-sm font-bold text-red-100">
                    <span className="h-2 w-2 rounded-full bg-red" />
                    Empezar con Rojo
                  </span>
                  <span className="mt-1 block text-xs text-muted">
                    Mismo formato, perspectiva roja
                  </span>
                  <span className="absolute right-4 top-1/2 -translate-y-1/2 text-xl text-red/70 transition group-hover:translate-x-0.5">
                    →
                  </span>
                </button>
              </div>

              <p className="mt-4 text-[11px] leading-relaxed text-muted/80">
                Sin winrates como argumento principal. Los huecos de información
                se muestran como pendientes, nunca como ceros o valores inventados.
              </p>

              <button
                type="button"
                onClick={onSoloq}
                className="group mt-4 flex w-full items-center gap-3 rounded-xl border border-gold/40 bg-gold/5 px-4 py-3 text-left transition hover:border-gold hover:bg-gold/10"
              >
                <span className="flex-1">
                  <span className="flex items-center gap-2 text-sm font-bold text-ink">
                    <span className="h-2 w-2 rounded-full bg-gold" />
                    Draft de SoloQ
                  </span>
                  <span className="mt-0.5 block text-xs text-muted">
                    Se sincroniza solo con el cliente durante el champ select y
                    te da el mejor pick, objetos y runas para tu turno.
                  </span>
                </span>
                <span className="text-xl text-gold/70 transition group-hover:translate-x-0.5">
                  →
                </span>
              </button>
            </div>
          </div>

          <div className="panel p-5">
            <div className="flex items-end justify-between gap-3">
              <div>
                <h2 className="text-base font-bold text-ink">Drafts recientes</h2>
                <p className="mt-1 text-xs text-muted">
                  Se guardan automáticamente tras cada acción.
                </p>
              </div>
              <span className="chip tabular">{sessions.length} guardados</span>
            </div>

            {sessions.length === 0 ? (
              <div className="mt-5 rounded-lg border border-dashed border-line px-4 py-8 text-center">
                <p className="text-sm font-semibold text-ink">
                  Todavía no hay drafts
                </p>
                <p className="mx-auto mt-1 max-w-md text-xs leading-relaxed text-muted">
                  Empieza uno con Azul o Rojo. Podrás deshacer, rehacer y
                  recuperarlo al cerrar la aplicación.
                </p>
              </div>
            ) : (
              <ul className="mt-4 divide-y divide-line/70">
                {sessions.map((session) => {
                  const progress =
                    session.totalActions > 0
                      ? (session.actionCount / session.totalActions) * 100
                      : 0;
                  return (
                    <li
                      key={session.id}
                      className="flex items-center gap-3 py-3 first:pt-1 last:pb-1"
                    >
                      <div
                        aria-hidden="true"
                        className={`h-8 w-1 rounded-full ${
                          session.side === "blue" ? "bg-blue" : "bg-red"
                        }`}
                      />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <SideBadge side={session.side} />
                          <span className="tabular text-xs text-muted">
                            {session.actionCount} / {session.totalActions} acciones
                          </span>
                          <span className="text-[11px] text-muted/70">
                            {formatRelative(session.updatedAt)}
                          </span>
                        </div>
                        <div className="mt-2 h-1 overflow-hidden rounded-full bg-surface2">
                          <div
                            className={`h-full rounded-full ${
                              session.side === "blue" ? "bg-blue" : "bg-red"
                            }`}
                            style={{ width: `${progress}%` }}
                          />
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => onOpenSession(session.id)}
                        disabled={busy}
                        className="btn-ghost px-3 py-1.5 text-xs"
                      >
                        Abrir
                      </button>
                      <button
                        type="button"
                        onClick={() => onDeleteSession(session.id)}
                        disabled={busy}
                        className="rounded-md px-2 py-1 text-xs text-muted transition hover:bg-redSoft/50 hover:text-red-200 disabled:opacity-40"
                        aria-label="Eliminar draft"
                        title="Eliminar draft"
                      >
                        ×
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </section>

        <aside className="flex flex-col gap-5 xl:col-span-4">
          <div className="panel p-5">
            <div className="flex items-center justify-between">
              <div>
                <p className="eyebrow">Datos locales</p>
                <h2 className="mt-2 text-base font-bold text-ink">
                  Estado del catálogo
                </h2>
              </div>
              <span className="text-3xl font-extrabold tabular text-ink">
                {data.championCount}
              </span>
            </div>

            <div className="mt-4 grid grid-cols-2 gap-2">
              <div className="rounded-lg border border-line/80 bg-bg/40 px-3 py-2">
                <p className="text-[10px] uppercase tracking-wider text-muted">
                  Con rol
                </p>
                <p className="mt-1 text-sm font-bold tabular text-success">
                  {data.championsWithRoles}
                </p>
              </div>
              <div className="rounded-lg border border-line/80 bg-bg/40 px-3 py-2">
                <p className="text-[10px] uppercase tracking-wider text-muted">
                  Rol pendiente
                </p>
                <p
                  className={`mt-1 text-sm font-bold tabular ${
                    data.championsWithoutRoles > 0 ? "text-warning" : "text-ink"
                  }`}
                >
                  {data.championsWithoutRoles}
                </p>
              </div>
            </div>

            <div className="mt-4">
              <div className="flex items-center justify-between text-[11px] text-muted">
                <span>Cobertura de roles</span>
                <span className="tabular">{coverage} %</span>
              </div>
              <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface2">
                <div
                  className="h-full rounded-full bg-success/80"
                  style={{ width: `${coverage}%` }}
                />
              </div>
            </div>

            <dl className="mt-4 space-y-2 text-xs">
              <div className="flex items-start justify-between gap-3">
                <dt className="text-muted">Identidad</dt>
                <dd className="text-right text-ink">{data.identitySource}</dd>
              </div>
              <div className="flex items-start justify-between gap-3">
                <dt className="text-muted">Roles</dt>
                <dd className="text-right text-ink">{data.rolesSource}</dd>
              </div>
              <div className="flex items-start justify-between gap-3">
                <dt className="text-muted">Taxonomía</dt>
                <dd className="text-right text-ink">
                  v{data.taxonomyVersion} · {data.archetypeCount} arq. ·{" "}
                  {data.traitCount} traits
                </dd>
              </div>
              <div className="flex items-start justify-between gap-3">
                <dt className="text-muted">Perfiles estratégicos</dt>
                <dd className="text-right text-warning">
                  {data.profileCount > 0
                    ? `${data.profileCount} perfiles · ${
                        data.activeProfileSet ?? "sin versión"
                      }`
                    : "0 · piloto pendiente"}
                </dd>
              </div>
              <div className="flex items-start justify-between gap-3">
                <dt className="text-muted">Última sincronización</dt>
                <dd className="text-right text-ink">
                  {data.lastSyncMs
                    ? formatRelative(data.lastSyncMs)
                    : "Semilla empaquetada"}
                </dd>
              </div>
            </dl>

            {data.warnings.length > 0 && (
              <ul className="mt-4 space-y-1.5 rounded-lg border border-warning/20 bg-warning/5 px-3 py-2.5">
                {data.warnings.map((warning) => (
                  <li
                    key={warning}
                    className="flex gap-2 text-[11px] leading-relaxed text-warning/90"
                  >
                    <span aria-hidden="true">•</span>
                    <span>{warning}</span>
                  </li>
                ))}
              </ul>
            )}

            {data.profileCount > 0 && (
              <div className="mt-4 rounded-lg border border-warning/30 bg-warning/5 px-3 py-2.5">
                <p className="text-[11px] font-semibold text-warning">
                  Conjunto activo: {data.activeProfileSet ?? "sin identificar"}
                </p>
                <p className="mt-1 text-[10px] leading-relaxed text-warning/90">
                  El análisis usa este conjunto. Las celdas marcadas{" "}
                  <strong>Inferred</strong> no son datos observados ni curados y
                  se muestran siempre como inferidas; las desconocidas no se
                  rellenan.
                </p>
              </div>
            )}

            <button
              type="button"
              onClick={onSync}
              disabled={busy || syncing || importingProfiles || importingSet}
              className="btn-ghost mt-4 w-full text-xs"
            >
              {syncing ? "Sincronizando…" : "Sincronizar Data Dragon"}
            </button>
            <p className="mt-1.5 text-center text-[10px] leading-relaxed text-muted/70">
              Actualiza identidades y conserva los roles ya importados.
            </p>
            <button
              type="button"
              onClick={onImportPilot}
              disabled={busy || syncing || importingProfiles || importingSet}
              className="btn-gold mt-2 w-full text-xs"
            >
              {importingProfiles
                ? "Cargando piloto…"
                : data.profileCount > 0
                  ? "Reimportar piloto inferido"
                  : "Cargar piloto inferido"}
            </button>
            <p className="mt-2 text-center text-[10px] leading-relaxed text-muted/70">
              Importa 38 perfiles de prueba y los activa. Sirven para validar el
              motor; no son datos definitivos.
            </p>

            <input
              ref={setFileInput}
              type="file"
              accept="application/json,.json"
              className="hidden"
              onChange={handleSetFile}
            />
            <button
              type="button"
              onClick={() => setFileInput.current?.click()}
              disabled={busy || syncing || importingProfiles || importingSet}
              className="btn-ghost mt-2 w-full text-xs"
            >
              {importingSet
                ? "Importando conjunto…"
                : "Importar conjunto revisado"}
            </button>
            <p className="mt-2 text-center text-[10px] leading-relaxed text-muted/70">
              Carga un fichero exportado por <code>loldraft-curate</code> y lo
              activa. La app no inventa celdas: cada una conserva su procedencia.
            </p>
          </div>

          <UpdatesCard currentVersion={bootstrap.appVersion} />

          <StatsDatasetsCard
            defaultPatch={data.patch}
            onNotify={onNotify}
          />

          <div className="panel p-5">
            <p className="eyebrow">Hoja de ruta</p>
            <ol className="mt-3 space-y-3">
              <li className="flex gap-3">
                <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-success/40 bg-success/10 text-[10px] font-bold text-success">
                  1
                </span>
                <div>
                  <p className="text-xs font-bold text-ink">
                    Simulador y persistencia
                  </p>
                  <p className="mt-0.5 text-[11px] leading-relaxed text-muted">
                    Disponible ahora: 20 acciones, undo/redo, roles óptimos y
                    sesiones locales.
                  </p>
                </div>
              </li>
              <li className="flex gap-3">
                <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-gold/40 bg-gold/10 text-[10px] font-bold text-gold">
                  2
                </span>
                <div>
                  <p className="text-xs font-bold text-ink">
                    Análisis de composición
                  </p>
                  <p className="mt-0.5 text-[11px] leading-relaxed text-muted">
                    Disponible con el piloto inferido: arquetipos, daño,
                    carencias, condiciones de victoria y sinergias derivadas.
                    Curadas, matchups observados y revisión experta, pendientes.
                  </p>
                </div>
              </li>
              <li className="flex gap-3">
                <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-success/40 bg-success/10 text-[10px] font-bold text-success">
                  3
                </span>
                <div>
                  <p className="text-xs font-bold text-ink">
                    Recomendaciones explicables
                  </p>
                  <p className="mt-0.5 text-[11px] leading-relaxed text-muted">
                    Picks, baneos, objetos y runas con motivos y confianza. El
                    loadout usa un catálogo curado de funciones y las amenazas
                    del rival; no necesita builds importadas.
                  </p>
                </div>
              </li>
            </ol>
          </div>
        </aside>
      </main>

      <footer className="flex flex-wrap items-center justify-between gap-2 border-t border-line/70 pt-4 text-[10px] text-muted/70">
        <p>
          LolDraft v{bootstrap.appVersion} · Rust + Tauri · Datos de Riot Data
          Dragon y Meraki Analytics
        </p>
        <p>No afiliado a Riot Games. League of Legends es propiedad de Riot Games.</p>
      </footer>
    </div>
  );
}
