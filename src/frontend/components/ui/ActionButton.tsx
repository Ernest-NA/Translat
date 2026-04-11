import type { ButtonHTMLAttributes, ReactNode } from "react";

type ActionButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ActionButtonSize = "sm" | "md";

interface ActionButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  mobileFullWidth?: boolean;
  size?: ActionButtonSize;
  variant?: ActionButtonVariant;
}

function buildClassName(
  className: string | undefined,
  mobileFullWidth: boolean,
  size: ActionButtonSize,
  variant: ActionButtonVariant,
) {
  const classes = ["action-button"];

  classes.push(`action-button--${variant}`);
  classes.push(`action-button--${size}`);

  if (mobileFullWidth) {
    classes.push("action-button--mobile-full-width");
  }

  if (className) {
    classes.push(className);
  }

  return classes.join(" ");
}

export function ActionButton({
  children,
  className,
  mobileFullWidth = false,
  size = "sm",
  type = "button",
  variant = "secondary",
  ...buttonProps
}: ActionButtonProps) {
  return (
    <button
      {...buttonProps}
      className={buildClassName(className, mobileFullWidth, size, variant)}
      type={type}
    >
      {children}
    </button>
  );
}
