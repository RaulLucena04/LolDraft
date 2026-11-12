import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import type {
  Bootstrap,
  Champion,
  CommandError,
  DraftView,
  Role,
  Side,
} from "./types";
import { Home } from "./components/Home";
import { DraftScreen } from "./components/DraftScreen";
import { ChampionDrawer } from "./components/ChampionDrawer";
import { ToastStack } from "./components/ToastStack";
import type { Toast } from "./components/ToastStack";
import { Logo } from "./components/Logo";

const IS_TAURI = "__TAURI_INTERNALS__" in window;

export default function App() {
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [draft, setDraft] = useState<DraftView | null>(null);
  const [screen, setScreen] = useState<"home" | "draft">("home");
  const [drawer, setDrawer] = useState<Champion | null>(null);
  const [selected, setSelected] = useState<Champion | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [fatal, setFatal] = useState<string | null>(null);
  const toastId = useRef(0);

  const showToast = useCallback(
    (kind: Toast["kind"], message: string) => {
      const id = ++toastId.current;
      setToasts((current) => [...current.slice(-3), { id, kind, message }]);
      window.setTimeout(() => {
        setToasts((current) => current.filter((toast) => toast.id !== id));
      }, 5200);
    },
    [],
  );

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const run = useCallback(
    async <T,>(operation: () => Promise<T>): Promise<T | null> => {
      setBusy(true);
      try {
        return await operation();
      } catch (error) {
        const commandError = error as Partial<CommandError>;
        showToast(
          "error",
          commandError.message ?? "Ocurrió un error inesperado.",
        );
        return null;
      } finally {
        setBusy(false);
      }
    },
    [showToast],
  );

  const refreshBootstrap = useCallback(async () => {
    if (!IS_TAURI) {
      setFatal(
        "Esta interfaz necesita ejecutarse dentro de la aplicación Tauri. Usa `npm run tauri dev` desde la raíz del proyecto.",
      );
      return;
    }
    const next = await run(() => api.bootstrap());
    if (next) {
      setBootstrap(next);
      setFatal(null);
    }
  }, [run]);

  useEffect(() => {
    void refreshBootstrap();
  }, [refreshBootstrap]);

  const handleNewDraft = useCallback(
    async (side: Side) => {
      const view = await run(() => api.newDraft(side));
      if (view) {
        setDraft(view);
        setSelected(null);
        setDrawer(null);
        setScreen("draft");
      }
    },
    [run],
  );

  const handleOpenSession = useCallback(
    async (id: string) => {
      const view = await run(() => api.load(id));
      if (view) {
        setDraft(view);
        setSelected(null);
        setDrawer(null);
        setScreen("draft");
      }
    },
    [run],
  );

  const handleDeleteSession = useCallback(
    async (id: string) => {
      if (
        !window.confirm(
          "¿Eliminar este draft guardado? Esta acción no se puede deshacer.",
        )
      ) {
        return;
      }
      const deleted = await run(() => api.deleteSession(id));
      if (deleted === null) return;
      showToast("info", "Draft eliminado.");
      if (draft?.id === id) {
        setDraft(null);
        setScreen("home");
        setSelected(null);
      }
      const next = await run(() => api.bootstrap());
      if (next) setBootstrap(next);
    },
    [run, draft?.id, showToast],
  );

  const handleAction = useCallback(
    async (championKey: number): Promise<boolean> => {
      if (!draft || busy) return false;
      const view = await run(() => api.action(championKey));
      if (!view) return false;
      setDraft(view);
      return true;
    },
    [draft, busy, run],
  );

  const handleUndo = useCallback(async () => {
    const view = await run(() => api.undo());
    if (view) {
      setDraft(view);
      setSelected(null);
    }
  }, [run]);

  const handleRedo = useCallback(async () => {
    const view = await run(() => api.redo());
    if (view) {
      setDraft(view);
      setSelected(null);
    }
  }, [run]);

  const handleSetRole = useCallback(
    async (championKey: number, role: Role | null) => {
      const view = await run(() => api.setRole(championKey, role));
      if (view) setDraft(view);
    },
    [run],
  );

  const handleHome = useCallback(() => {
    setScreen("home");
    setDrawer(null);
    setSelected(null);
    void (async () => {
      try {
        const next = await api.bootstrap();
        setBootstrap(next);
      } catch {
        // La pantalla de inicio conserva los datos ya cargados.
      }
    })();
  }, []);

  const handleSync = useCallback(async () => {
    setSyncing(true);
    try {
      const status = await api.syncDdragon();
      const next = await api.bootstrap();
      setBootstrap(next);
      showToast(
        "success",
        `Catálogo sincronizado: ${status.championCount} campeones${
          status.patch ? ` · parche ${status.patch}` : ""
        }.`,
      );
    } catch (error) {
      const commandError = error as Partial<CommandError>;
      showToast(
        "error",
        commandError.message ?? "No se pudo sincronizar Data Dragon.",
      );
    } finally {
      setSyncing(false);
    }
  }, [showToast]);

  const handleInspect = useCallback(
    (championKey: number) => {
      const champion = bootstrap?.champions.find(
        (item) => item.key === championKey,
      );
      if (champion) setDrawer(champion);
    },
    [bootstrap],
  );

  const handleChooseFromDrawer = useCallback((champion: Champion) => {
    setSelected(champion);
    setDrawer(null);
  }, []);

  if (fatal) {
    return (
      <div className="flex min-h-screen items-center justify-center px-6">
        <div className="panel w-full max-w-lg p-7 text-center">
          <div className="flex justify-center">
            <Logo />
          </div>
          <h1 className="mt-6 text-xl font-extrabold text-ink">
            No se pudo iniciar el catálogo
          </h1>
          <p className="mt-3 text-sm leading-relaxed text-muted">{fatal}</p>
          <button
            type="button"
            onClick={() => void refreshBootstrap()}
            className="btn-gold mt-6"
          >
            Reintentar
          </button>
        </div>
      </div>
    );
  }

  if (!bootstrap) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center gap-5">
        <Logo />
        <div className="flex items-center gap-2 text-xs text-muted">
          <span
            aria-hidden="true"
            className="h-2 w-2 animate-pulse rounded-full bg-gold"
          />
          Cargando catálogo local y sesiones…
        </div>
      </div>
    );
  }

  return (
    <>
      {screen === "draft" && draft ? (
        <DraftScreen
          draft={draft}
          champions={bootstrap.champions}
          data={bootstrap.data}
          busy={busy}
          selected={selected}
          onSelect={setSelected}
          onAction={handleAction}
          onUndo={() => void handleUndo()}
          onRedo={() => void handleRedo()}
          onSetRole={(key, role) => void handleSetRole(key, role)}
          onHome={handleHome}
          onInspect={handleInspect}
        />
      ) : (
        <Home
          bootstrap={bootstrap}
          busy={busy}
          syncing={syncing}
          onNewDraft={(side) => void handleNewDraft(side)}
          onOpenSession={(id) => void handleOpenSession(id)}
          onDeleteSession={(id) => void handleDeleteSession(id)}
          onSync={() => void handleSync()}
        />
      )}

      <ChampionDrawer
        champion={drawer}
        busy={busy}
        chooseBlockedReason={
          !drawer
            ? null
            : !draft
              ? "Abre un draft para elegir campeones."
              : !draft.nextStep
                ? "El draft está completo: ya no quedan turnos."
                : draft.steps.some(
                      (step) =>
                        step.status === "done" &&
                        step.champion?.key === drawer.key,
                    )
                  ? "Este campeón ya está usado en este draft."
                  : null
        }
        onClose={() => setDrawer(null)}
        onChoose={handleChooseFromDrawer}
      />

      <ToastStack toasts={toasts} onDismiss={dismissToast} />
    </>
  );
}
