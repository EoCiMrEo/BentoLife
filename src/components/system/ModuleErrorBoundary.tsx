import type { ReactNode } from "react";

import { ErrorBoundary } from "@/components/system/ErrorBoundary";

export function ModuleErrorBoundary({
  children,
  moduleLabel,
  onRetry,
  recoveryAction,
}: {
  children: ReactNode;
  moduleLabel: string;
  onRetry?: () => void;
  recoveryAction?: ReactNode;
}) {
  return (
    <ErrorBoundary
      fallbackTitle={`${moduleLabel} could not render`}
      onRetry={onRetry}
      recoveryAction={recoveryAction}
    >
      {children}
    </ErrorBoundary>
  );
}
