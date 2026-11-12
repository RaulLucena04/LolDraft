export function Logo({ compact = false }: { compact?: boolean }) {
  return (
    <div className="flex items-center gap-2.5">
      <svg
        width="30"
        height="30"
        viewBox="0 0 32 32"
        role="img"
        aria-label="LolDraft"
        className="shrink-0"
      >
        <defs>
          <clipPath id="ld-mark">
            <path d="M16 1.5 30.5 16 16 30.5 1.5 16Z" />
          </clipPath>
        </defs>
        <path
          d="M16 1.5 30.5 16 16 30.5 1.5 16Z"
          fill="#111823"
          stroke="#D4AF37"
          strokeWidth="1.4"
        />
        <g clipPath="url(#ld-mark)">
          <path d="M0 0h18.5L10 32H0Z" fill="#1E3A5F" />
          <path d="M18.5 0H32v32H10Z" fill="#4A1D1D" />
        </g>
        <path
          d="M16 5.5 26.5 16 16 26.5 5.5 16Z"
          fill="none"
          stroke="#D4AF37"
          strokeWidth="1"
          opacity="0.8"
        />
        <text
          x="16"
          y="20.4"
          textAnchor="middle"
          fontSize="9.5"
          fontWeight="800"
          fill="#E6EDF5"
          fontFamily="Segoe UI, system-ui, sans-serif"
        >
          LD
        </text>
      </svg>
      {!compact && (
        <div className="leading-none">
          <p className="text-[15px] font-extrabold tracking-[0.12em] text-ink">
            LOL<span className="text-gold">DRAFT</span>
          </p>
          <p className="mt-1 text-[9px] font-semibold uppercase tracking-[0.22em] text-muted">
            Draft assistant
          </p>
        </div>
      )}
    </div>
  );
}
