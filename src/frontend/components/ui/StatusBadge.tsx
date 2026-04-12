import type { ReactNode } from "react";

type StatusBadgeEmphasis = "soft" | "strong";
type StatusBadgeSize = "sm" | "md";
type StatusBadgeTone = "neutral" | "info" | "success" | "warning" | "danger";

interface StatusBadgeProps {
  children: ReactNode;
  className?: string;
  emphasis?: StatusBadgeEmphasis;
  size?: StatusBadgeSize;
  tone?: StatusBadgeTone;
}

function buildClassName(className?: string) {
  return className ? `status-badge ${className}` : "status-badge";
}

export function StatusBadge({
  children,
  className,
  emphasis = "soft",
  size = "sm",
  tone = "neutral",
}: StatusBadgeProps) {
  return (
    <span
      className={buildClassName(className)}
      data-emphasis={emphasis}
      data-size={size}
      data-tone={tone}
    >
      {children}
    </span>
  );
}
