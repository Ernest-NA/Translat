import type { HTMLAttributes, ReactNode } from "react";

type PanelMessageTone = "neutral" | "info" | "success" | "warning" | "danger";

interface PanelMessageProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  title?: string;
  tone?: PanelMessageTone;
}

function buildClassName(className: string | undefined) {
  return className ? `panel-message ${className}` : "panel-message";
}

export function PanelMessage({
  children,
  className,
  title,
  tone = "neutral",
  ...divProps
}: PanelMessageProps) {
  return (
    <div {...divProps} className={buildClassName(className)} data-tone={tone}>
      {title ? <strong className="panel-message__title">{title}</strong> : null}
      <div className="panel-message__body">{children}</div>
    </div>
  );
}
