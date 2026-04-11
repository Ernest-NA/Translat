import type { ReactNode } from "react";

interface PanelHeaderProps {
  actions?: ReactNode;
  description?: ReactNode;
  eyebrow: string;
  meta?: ReactNode;
  title: string;
  titleLevel?: 2 | 3;
}

export function PanelHeader({
  actions,
  description,
  eyebrow,
  meta,
  title,
  titleLevel = 3,
}: PanelHeaderProps) {
  const TitleTag = titleLevel === 2 ? "h2" : "h3";

  return (
    <div className="panel-header">
      <div className="panel-header__copy">
        <p className="surface-card__eyebrow">{eyebrow}</p>
        <TitleTag>{title}</TitleTag>
        {description ? (
          <p className="panel-header__description">{description}</p>
        ) : null}
      </div>

      {actions || meta ? (
        <div className="panel-header__aside">
          {meta ? <div className="panel-header__meta">{meta}</div> : null}
          {actions ? (
            <div className="panel-header__actions">{actions}</div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
