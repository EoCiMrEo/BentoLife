import type { ReactNode } from "react";

import { ErrorBoundary } from "@/components/system/ErrorBoundary";

export function WidgetErrorBoundary({
  children,
  widgetLabel,
}: {
  children: ReactNode;
  widgetLabel: string;
}) {
  return (
    <ErrorBoundary fallbackTitle={`${widgetLabel} widget could not render`}>
      {children}
    </ErrorBoundary>
  );
}
