import React from "react";

import { Button } from "@/components/ui/button";

type ErrorBoundaryProps = {
  children: React.ReactNode;
  fallbackTitle: string;
  onRetry?: () => void;
  recoveryAction?: React.ReactNode;
};

type ErrorBoundaryState = {
  error: Error | null;
};

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error) {
    if (typeof console !== "undefined") {
      console.error("[BentoLife ErrorBoundary]", error);
    }
  }

  render() {
    const { children, fallbackTitle, onRetry, recoveryAction } = this.props;
    const { error } = this.state;
    if (!error) {
      return children;
    }

    const retry = onRetry
      ? () => {
          this.setState({ error: null });
          onRetry();
        }
      : undefined;

    return (
      <div className="grid gap-3 rounded-md border border-border bg-muted/45 p-4 text-sm">
        <div>
          <p className="font-medium">{fallbackTitle}</p>
          <p className="mt-1 text-muted-foreground">This area is temporarily isolated so the rest of BentoLife stays usable.</p>
        </div>
        <details className="rounded-md border border-border bg-background p-3">
          <summary className="cursor-pointer text-xs font-semibold text-muted-foreground">Technical detail</summary>
          <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap break-words text-xs">{error.stack ?? error.message}</pre>
        </details>
        <div className="flex flex-wrap gap-2">
          {retry ? <Button onClick={retry} size="sm" variant="outline">Retry</Button> : null}
          <Button
            onClick={() => void navigator.clipboard?.writeText(error.stack ?? error.message)}
            size="sm"
            variant="ghost"
          >
            Copy details
          </Button>
          {recoveryAction}
        </div>
      </div>
    );
  }
}
