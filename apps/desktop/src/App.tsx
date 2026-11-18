import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import type {
  Bootstrap,
  Champion,
  ChampionProfileDetail,
  CommandError,
  DraftAnalysis,
  DraftView,
  RecommendationSet,
  Role,
  Side,
} from "./types";
import { Home } from "./components/Home";
import { DraftScreen } from "./components/DraftScreen";
import { SoloqScreen } from "./components/SoloqScreen";
import { ChampionDrawer } from "./components/ChampionDrawer";
import { ToastStack } from "./components/ToastStack";
import type { Toast } from "./components/ToastStack";
import { Logo } from "./components/Logo";

const IS_TAURI = "__TAURI_INTERNALS__" in window;

export default function App() {
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [draft, setDraft] = useState<DraftView | null>(null);
  const [screen, setScreen] = useState<"home" | "draft" | "soloq">("home");
  const [drawer, setDrawer] = useState<Champion | null>(null);
  const [selected, setSelected] = useState<Champion | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [importingProfiles, setImportingProfiles] = useState(false);
  const [importingSet, setImportingSet] = useState(false);
  const [analysis, setAnalysis] = useState<DraftAnalysis | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [recommendationData, setRecommendationData] = useState<{
    key: string;
    set: RecommendationSet;
  } | null>(null);
  const [recommendationError, setRecommendationError] = useState<string | null>(
    null,
  );
  const [recommending, setRecommending] = useState(false);
  const [championProfile, setChampionProfile] =
    useState<ChampionProfileDetail | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [fatal, setFatal] = useState<string | null>(null);
  const toastId = useRef(0);
  const inspectSeq = useRef(0);
  const recommendationSeq = useRef(0);

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
        setAnalysis(null);
        setAnalysisError(null);
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
        setAnalysis(null);
        setAnalysisError(null);
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
        setAnalysis(null);
        setAnalysisError(null);
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
      setAnalysis(null);
      setAnalysisError(null);
      return true;
    },
    [draft, busy, run],
  );

  const handleUndo = useCallback(async () => {
    const view = await run(() => api.undo());
    if (view) {
      setDraft(view);
      setSelected(null);
      setAnalysis(null);
      setAnalysisError(null);
    }
  }, [run]);

  const handleRedo = useCallback(async () => {
    const view = await run(() => api.redo());
    if (view) {
      setDraft(view);
      setSelected(null);
      setAnalysis(null);
      setAnalysisError(null);
    }
  }, [run]);

  const handleSetRole = useCallback(
    async (championKey: number, role: Role | null) => {
      const view = await run(() => api.setRole(championKey, role));
      if (view) {
        setDraft(view);
        setAnalysis(null);
        setAnalysisError(null);
      }
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

  const handleSoloq = useCallback(() => {
    setScreen("soloq");
    setDrawer(null);
    setSelected(null);
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

  const handleImportPilot = useCallback(async () => {
    setImportingProfiles(true);
    try {
      const set = await api.importPilot(true);
      const next = await api.bootstrap();
      setBootstrap(next);
      setAnalysis(null);
      setAnalysisError(null);
      showToast(
        "success",
        `Piloto inferido ${set.version} cargado y activo. Sus celdas son Inferred y quedan pendientes de revisión.`,
      );
    } catch (error) {
      const commandError = error as Partial<CommandError>;
      showToast(
        "error",
        commandError.message ?? "No se pudo cargar el piloto inferido.",
      );
    } finally {
      setImportingProfiles(false);
    }
  }, [showToast]);

  const handleImportSet = useCallback(
    async (file: File) => {
      setImportingSet(true);
      try {
        const text = await file.text();
        const set = await api.importProfileSet(text, true);
        const next = await api.bootstrap();
        setBootstrap(next);
        setAnalysis(null);
        setAnalysisError(null);
        showToast(
          "success",
          `Conjunto ${set.id} · ${set.version} importado y activo.`,
        );
      } catch (error) {
        const commandError = error as Partial<CommandError>;
        showToast(
          "error",
          commandError.message ??
            "No se pudo importar el conjunto de perfiles.",
        );
      } finally {
        setImportingSet(false);
      }
    },
    [showToast],
  );

  const handleAnalyze = useCallback(async () => {
    setAnalyzing(true);
    setAnalysisError(null);
    try {
      const next = await api.analysis();
      setAnalysis(next);
    } catch (error) {
      const commandError = error as Partial<CommandError>;
      setAnalysis(null);
      setAnalysisError(
        commandError.message ?? "No se pudo analizar la composición.",
      );
    } finally {
      setAnalyzing(false);
    }
  }, []);

  const handleInspect = useCallback(
    (championKey: number) => {
      const champion = bootstrap?.champions.find(
        (item) => item.key === championKey,
      );
      if (!champion) return;
      setDrawer(champion);
      setChampionProfile(null);
      setProfileError(null);
      setProfileLoading(true);
      const seq = ++inspectSeq.current;
      void (async () => {
        try {
          const detail = await api.championProfile(championKey);
          if (seq === inspectSeq.current) setChampionProfile(detail);
        } catch (error) {
          if (seq !== inspectSeq.current) return;
          const commandError = error as Partial<CommandError>;
          setProfileError(
            commandError.message ??
              "No se pudo cargar el perfil del campeón.",
          );
        } finally {
          if (seq === inspectSeq.current) setProfileLoading(false);
        }
      })();
    },
    [bootstrap],
  );

  const handleCloseDrawer = useCallback(() => {
    inspectSeq.current += 1;
    setDrawer(null);
    setChampionProfile(null);
    setProfileError(null);
    setProfileLoading(false);
  }, []);

  const handleChooseFromDrawer = useCallback((champion: Champion) => {
    setSelected(champion);
    setDrawer(null);
  }, []);

  const hasProfiles = (bootstrap?.data.profileCount ?? 0) > 0;
  const activeProfileSet = bootstrap?.data.activeProfileSet ?? "";
  const assignmentKey = draft
    ? [...draft.teams.blue.picks, ...draft.teams.red.picks]
        .map(
          (pick) =>
            `${pick.champion.key}:${pick.role ?? "-"}:${pick.roleSource}`,
        )
        .join("|")
    : "";
  const recommendationKey = draft
    ? `${draft.id}:${draft.actionCount}:${activeProfileSet}:${assignmentKey}`
    : null;
  const recommendations =
    recommendationData && recommendationData.key === recommendationKey
      ? recommendationData.set
      : null;

  const handleRecommendations = useCallback(async () => {
    if (!recommendationKey) return;
    const seq = ++recommendationSeq.current;
    setRecommending(true);
    setRecommendationError(null);
    try {
      const next = await api.recommendations();
      if (seq !== recommendationSeq.current) return;
      setRecommendationData({ key: recommendationKey, set: next });
    } catch (error) {
      if (seq !== recommendationSeq.current) return;
      const commandError = error as Partial<CommandError>;
      setRecommendationData(null);
      setRecommendationError(
        commandError.message ?? "No se pudieron calcular las recomendaciones.",
      );
    } finally {
      if (seq === recommendationSeq.current) setRecommending(false);
    }
  }, [recommendationKey]);

  useEffect(() => {
    if (
      screen !== "draft" ||
      !recommendationKey ||
      !hasProfiles ||
      draft?.isComplete
    ) {
      return;
    }
    void handleRecommendations();
  }, [
    screen,
    recommendationKey,
    hasProfiles,
    draft?.isComplete,
    handleRecommendations,
  ]);

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
          analysis={analysis}
          analyzing={analyzing}
          analysisError={analysisError}
          onAnalyze={() => void handleAnalyze()}
          recommendations={draft.isComplete ? null : recommendations}
          recommending={recommending}
          recommendationError={recommendationError}
          onRefreshRecommendations={() => void handleRecommendations()}
        />
      ) : screen === "soloq" ? (
        <SoloqScreen
          champions={bootstrap.champions}
          hasProfiles={hasProfiles}
          onHome={handleHome}
          onInspect={handleInspect}
        />
      ) : (
        <Home
          bootstrap={bootstrap}
          busy={busy}
          syncing={syncing}
          importingProfiles={importingProfiles}
          importingSet={importingSet}
          onNewDraft={(side) => void handleNewDraft(side)}
          onSoloq={handleSoloq}
          onOpenSession={(id) => void handleOpenSession(id)}
          onDeleteSession={(id) => void handleDeleteSession(id)}
          onSync={() => void handleSync()}
          onImportPilot={() => void handleImportPilot()}
          onImportSet={(file) => void handleImportSet(file)}
        />
      )}

      <ChampionDrawer
        champion={drawer}
        profile={championProfile}
        profileLoading={profileLoading}
        profileError={profileError}
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
        onClose={handleCloseDrawer}
        onChoose={handleChooseFromDrawer}
      />

      <ToastStack toasts={toasts} onDismiss={dismissToast} />
    </>
  );
}
