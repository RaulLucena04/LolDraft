import type { DraftView } from "../types";
import { ChampionAvatar } from "./ChampionAvatar";
import { KIND_LABEL, SIDE_LABEL } from "../lib/format";

const PHASE_BREAK = new Set([6, 12, 16]);

export function Timeline({ draft }: { draft: DraftView }) {
  return (
    <section className="panel flex items-center gap-3 px-3 py-2">
      <div className="shrink-0 border-r border-line/70 pr-3">
        <p className="eyebrow">Secuencia</p>
        <div className="mt-1 flex items-center gap-1.5">
          <span className="text-xs font-bold text-ink">
            {draft.totalActions} turnos
          </span>
          <span className="chip tabular">
            {draft.actionCount} / {draft.totalActions}
          </span>
        </div>
      </div>

      <ol className="flex min-w-0 flex-1 items-stretch gap-1 overflow-x-auto pb-0.5">
        {draft.steps.map((step) => {
          const current = step.status === "current";
          const done = step.status === "done";
          return (
            <li
              key={step.index}
              title={`${step.number}. ${SIDE_LABEL[step.side]} · ${
                KIND_LABEL[step.kind]
              }${step.champion ? ` · ${step.champion.name}` : " · pendiente"}`}
              aria-current={current ? "step" : undefined}
              className={`flex min-w-[38px] flex-1 flex-col items-center justify-center gap-0.5 rounded-md border px-0.5 py-1 ${
                PHASE_BREAK.has(step.index) ? "ml-1.5" : ""
              } ${
                current
                  ? "border-gold/70 bg-gold/10 shadow-glow"
                  : done
                    ? step.side === "blue"
                      ? "border-line/60 border-b-2 border-b-blue/70 bg-bg/40"
                      : "border-line/60 border-b-2 border-b-red/70 bg-bg/40"
                    : "border-line/40 bg-bg/20 opacity-55"
              }`}
            >
              <span
                className={`tabular text-[9px] font-bold leading-none ${
                  current ? "text-gold" : "text-muted/70"
                }`}
              >
                {step.number}
              </span>
              {step.champion ? (
                <ChampionAvatar
                  champion={step.champion}
                  size="xs"
                  dimmed={step.kind === "ban"}
                />
              ) : (
                <span
                  className={`flex h-8 w-8 items-center justify-center rounded-md border text-[9px] font-bold uppercase ${
                    current
                      ? "border-gold/40 text-gold"
                      : step.side === "blue"
                        ? "border-line/50 text-blue/70"
                        : "border-line/50 text-red/70"
                  }`}
                >
                  {step.kind === "ban" ? "B" : "P"}
                </span>
              )}
            </li>
          );
        })}
      </ol>
    </section>
  );
}
