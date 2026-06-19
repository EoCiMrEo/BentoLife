import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export type WidgetDensity = "compact" | "standard" | "expanded";

export function widgetDensity(size: { width: number; height: number }): WidgetDensity {
  if (size.width >= 4 || size.height >= 2) return "expanded";
  if (size.width <= 1 || size.height <= 1) return "compact";
  return "standard";
}

export function WidgetFrame({ children, className }: { children: ReactNode; className?: string }) {
  return <WidgetSurface className={className}>{children}</WidgetSurface>;
}

export function WidgetSurface({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("flex h-full min-h-0 flex-col gap-3", className)}>{children}</div>;
}

export function WidgetIconHeader({
  action,
  badge,
  icon,
  subtitle,
  title,
}: {
  action?: ReactNode;
  badge?: string;
  icon?: ReactNode;
  subtitle?: string;
  title: ReactNode;
}) {
  return (
    <div className="flex shrink-0 items-start justify-between gap-3">
      <div className="flex min-w-0 items-start gap-2">
        {icon ? (
          <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
            {icon}
          </span>
        ) : null}
        <div className="min-w-0">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <p className="min-w-0 truncate text-sm font-semibold">{title}</p>
            {badge ? <WidgetModuleBadge label={badge} /> : null}
          </div>
          {subtitle ? <p className="mt-0.5 truncate text-xs text-muted-foreground">{subtitle}</p> : null}
        </div>
      </div>
      {action ? <div className="shrink-0">{action}</div> : null}
    </div>
  );
}

export function WidgetModuleBadge({ label }: { label: string }) {
  return (
    <span className="max-w-28 truncate rounded-full border border-border bg-muted/35 px-2 py-0.5 text-[10px] font-medium uppercase text-muted-foreground">
      {label}
    </span>
  );
}

export function WidgetContentScrollArea({
  children,
  className,
  maxRows = 4,
}: {
  children: ReactNode;
  className?: string;
  maxRows?: number;
}) {
  return (
    <div
      className={cn(
        "relative min-h-0 flex-1 overflow-y-auto overflow-x-hidden pr-1 [scrollbar-width:thin]",
        "after:pointer-events-none after:sticky after:bottom-0 after:block after:h-4 after:bg-gradient-to-t after:from-card after:to-transparent",
        className,
      )}
      data-widget-scroll-area
      style={{ maxHeight: `${Math.max(1, maxRows) * 3.25}rem` }}
    >
      {children}
    </div>
  );
}

export function WidgetEmptyIllustration({ children }: { children?: ReactNode }) {
  return (
    <span className="mb-3 flex size-9 items-center justify-center rounded-md bg-primary/10 text-primary" aria-hidden="true">
      {children ?? "B"}
    </span>
  );
}

export function WidgetEmptyState({
  action,
  description,
  illustration,
  title,
}: {
  action?: ReactNode;
  description?: string;
  illustration?: ReactNode;
  title: string;
}) {
  return (
    <div className="rounded-md border border-dashed border-border bg-muted/35 p-3 text-sm">
      {illustration ? <WidgetEmptyIllustration>{illustration}</WidgetEmptyIllustration> : null}
      <p className="font-medium">{title}</p>
      {description ? <p className="mt-1 text-muted-foreground">{description}</p> : null}
      {action ? <div className="mt-3">{action}</div> : null}
    </div>
  );
}

export function WidgetList({
  items,
  maxRows,
}: {
  items: Array<{
    action?: ReactNode;
    detail?: string;
    label: string;
    leading?: ReactNode;
    onClick?: () => void;
    tone?: "default" | "warning";
  }>;
  maxRows?: number;
}) {
  return (
    <WidgetContentScrollArea maxRows={maxRows}>
      <div className="flex flex-col gap-2">
      {items.map((item) => {
        return (
          <WidgetRow
            action={item.action}
            detail={item.detail}
            key={`${item.label}-${item.detail ?? ""}`}
            label={item.label}
            leading={item.leading}
            onClick={item.onClick}
            tone={item.tone}
          />
        );
      })}
      </div>
    </WidgetContentScrollArea>
  );
}

export function WidgetRow({
  action,
  detail,
  label,
  leading,
  onClick,
  tone = "default",
}: {
  action?: ReactNode;
  detail?: string;
  label: string;
  leading?: ReactNode;
  onClick?: () => void;
  tone?: "default" | "warning";
}) {
  const textContent = (
    <>
      {leading ? <span className="shrink-0">{leading}</span> : null}
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{label}</span>
        {detail ? <span className="block truncate text-xs text-muted-foreground">{detail}</span> : null}
      </span>
    </>
  );
  const className = cn(
    "flex min-w-0 items-center gap-2 rounded-md px-3 py-2 text-left",
    tone === "warning" ? "bg-amber-note" : "bg-muted/45",
    onClick ? "transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" : "",
  );
  if (action) {
    return (
      <div className={className} data-widget-row>
        {onClick ? (
          <button className="flex min-w-0 flex-1 items-center gap-2 text-left" onClick={onClick} type="button">
            {textContent}
          </button>
        ) : (
          textContent
        )}
        <span className="shrink-0" onClick={(event) => event.stopPropagation()}>
          {action}
        </span>
      </div>
    );
  }
  return onClick ? (
    <button className={className} data-widget-row onClick={onClick} type="button">
      {textContent}
    </button>
  ) : (
    <div className={className} data-widget-row>
      {textContent}
    </div>
  );
}

export function WidgetProgress({ label, percent }: { label: string; percent: number }) {
  const boundedPercent = Math.max(0, Math.min(100, Math.round(percent)));
  return (
    <div className="space-y-2">
      <div className="h-2 overflow-hidden rounded-full bg-muted">
        <div className="h-full rounded-full bg-primary transition-[width]" style={{ width: `${boundedPercent}%` }} />
      </div>
      <p className="text-sm text-muted-foreground">{label}</p>
    </div>
  );
}

export function WidgetTimeline({
  items,
}: {
  items: Array<{ date?: string; label: string; onClick?: () => void }>;
}) {
  return (
    <ol className="flex flex-col gap-2">
      {items.map((item) => (
        <li className="flex gap-2" key={`${item.label}-${item.date ?? ""}`}>
          <span className="mt-2 size-2 shrink-0 rounded-full bg-primary" />
          <button
            className="min-w-0 text-left text-sm hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            disabled={!item.onClick}
            onClick={item.onClick}
            type="button"
          >
            <span className="block truncate font-medium">{item.label}</span>
            {item.date ? <span className="block text-xs text-muted-foreground">{item.date}</span> : null}
          </button>
        </li>
      ))}
    </ol>
  );
}
