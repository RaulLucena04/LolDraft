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
          fill="#0A1428"
          stroke="#C89B3C"
          strokeWidth="1.4"
        />
        <g clipPath="url(#ld-mark)">
          <path d="M0 0h18.5L10 32H0Z" fill="#06322F" />
          <path d="M18.5 0H32v32H10Z" fill="#3A121B" />
        </g>
        <path
          d="M16 5.5 26.5 16 16 26.5 5.5 16Z"
          fill="none"
          stroke="#C89B3C"
          strokeWidth="1"
          opacity="0.75"
        />
        <text
          x="16"
          y="20.4"
          textAnchor="middle"
          fontSize="9"
          fontWeight="700"
          letterSpacing="0.4"
          fill="#F0E6D2"
          fontFamily="Cinzel, Georgia, serif"
        >
          LD
        </text>
      </svg>
      {!compact && (
        <div className="leading-none">
          <p className="font-display text-[15px] font-bold tracking-[0.14em] text-ink">
            LOL<span className="text-gold">DRAFT</span>
          </p>
          <p className="mt-1 text-[9px] font-semibold uppercase tracking-[0.24em] text-muted">
            Draft assistant
          </p>
        </div>
      )}
    </div>
  );
}
