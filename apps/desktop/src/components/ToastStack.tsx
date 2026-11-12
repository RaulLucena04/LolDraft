export interface Toast {
  id: number;
  kind: "success" | "error" | "info";
  message: string;
}

interface ToastStackProps {
  toasts: Toast[];
  onDismiss: (id: number) => void;
}

export function ToastStack({ toasts, onDismiss }: ToastStackProps) {
  return (
    <div
      className="pointer-events-none fixed right-5 top-5 z-50 flex w-[360px] max-w-[calc(100vw-2rem)] flex-col gap-2"
      role="status"
      aria-live="polite"
    >
      {toasts.map((toast) => {
        const tone =
          toast.kind === "success"
            ? "border-success/45 bg-[#0f2419]"
            : toast.kind === "error"
              ? "border-danger/50 bg-[#2a1113]"
              : "border-blue/45 bg-[#0f1d31]";
        return (
          <div
            key={toast.id}
            className={`pointer-events-auto animate-rise rounded-lg border px-3 py-2.5 text-sm shadow-panel ${tone}`}
          >
            <div className="flex items-start gap-2">
              <span className="mt-0.5 text-xs">
                {toast.kind === "success" ? "✓" : toast.kind === "error" ? "!" : "i"}
              </span>
              <p className="flex-1 leading-snug text-ink">{toast.message}</p>
              <button
                type="button"
                onClick={() => onDismiss(toast.id)}
                className="rounded px-1 text-muted hover:text-ink"
                aria-label="Cerrar aviso"
              >
                ×
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
