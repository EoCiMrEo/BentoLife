import { X } from "lucide-react";
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export type AppNoticeKind = "success" | "info" | "warning" | "error";

export type ToastInput = {
  kind?: AppNoticeKind;
  message: string;
  title?: string;
};

type ToastItem = Required<ToastInput> & {
  id: string;
};

type ToastContextValue = {
  dismissToast: (id: string) => void;
  showToast: (toast: ToastInput) => string;
};

const ToastContext = createContext<ToastContextValue | null>(null);

const autoDismissMs = 4000;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const dismissToast = useCallback((id: string) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const showToast = useCallback((toast: ToastInput) => {
    const id = typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const nextToast: ToastItem = {
      id,
      kind: toast.kind ?? "info",
      message: toast.message,
      title: toast.title ?? defaultToastTitle(toast.kind ?? "info"),
    };
    setToasts((current) => [...current, nextToast]);
    if (nextToast.kind === "success" || nextToast.kind === "info") {
      window.setTimeout(() => dismissToast(id), autoDismissMs);
    }
    return id;
  }, [dismissToast]);

  const value = useMemo(() => ({ dismissToast, showToast }), [dismissToast, showToast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <ToastViewport dismissToast={dismissToast} toasts={toasts} />
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used inside ToastProvider");
  }
  return context;
}

function ToastViewport({ dismissToast, toasts }: { dismissToast: (id: string) => void; toasts: ToastItem[] }) {
  if (!toasts.length) {
    return null;
  }

  return (
    <div
      aria-live="polite"
      aria-relevant="additions removals"
      className="fixed bottom-4 right-4 z-50 flex w-[min(calc(100vw-2rem),24rem)] flex-col gap-2"
      role="status"
    >
      {toasts.map((toast) => (
        <div
          className={cn(
            "rounded-md border bg-card p-3 text-sm shadow-lg",
            toast.kind === "success" && "border-emerald-500/35",
            toast.kind === "info" && "border-sky-500/35",
            toast.kind === "warning" && "border-amber-note/60",
            toast.kind === "error" && "border-destructive/50",
          )}
          key={toast.id}
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="font-semibold">{toast.title}</p>
              <p className="mt-1 break-words text-muted-foreground">{toast.message}</p>
            </div>
            <Button aria-label={`Dismiss ${toast.title}`} onClick={() => dismissToast(toast.id)} size="icon" variant="ghost">
              <X className="size-4" />
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}

function defaultToastTitle(kind: AppNoticeKind) {
  switch (kind) {
    case "success":
      return "Done";
    case "warning":
      return "Needs attention";
    case "error":
      return "Action failed";
    case "info":
    default:
      return "BentoLife";
  }
}
