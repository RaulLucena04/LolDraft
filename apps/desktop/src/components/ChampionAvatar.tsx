import { useState } from "react";

type AvatarSize = "xs" | "sm" | "md" | "lg" | "xl" | "fill";

const SIZE_CLASS: Record<AvatarSize, string> = {
  xs: "h-8 w-8 text-[10px]",
  sm: "h-10 w-10 text-xs",
  md: "h-12 w-12 text-sm",
  lg: "h-16 w-16 text-lg",
  xl: "h-24 w-24 text-2xl",
  fill: "h-full w-full text-sm",
};

interface ChampionAvatarProps {
  champion: { name: string; imagePath: string };
  size?: AvatarSize;
  className?: string;
  dimmed?: boolean;
}

export function ChampionAvatar({
  champion,
  size = "md",
  className = "",
  dimmed = false,
}: ChampionAvatarProps) {
  const [failed, setFailed] = useState(false);
  const initials = champion.name.slice(0, 2).toUpperCase();

  return (
    <span
      className={`relative inline-flex shrink-0 items-center justify-center overflow-hidden rounded-md border border-line/90 bg-surface2 font-bold text-muted ${
        SIZE_CLASS[size]
      } ${dimmed ? "opacity-45 grayscale" : ""} ${className}`}
    >
      {failed ? (
        <span aria-hidden="true">{initials}</span>
      ) : (
        <img
          key={champion.imagePath}
          src={champion.imagePath}
          alt={champion.name}
          loading="lazy"
          decoding="async"
          draggable={false}
          onError={() => setFailed(true)}
          className="h-full w-full object-cover"
        />
      )}
    </span>
  );
}
