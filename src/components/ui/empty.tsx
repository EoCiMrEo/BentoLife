import { FileText } from "lucide-react";
import type React from "react";

import { cn } from "@/lib/utils";

type EmptyProps = {
  title: string;
  description: string;
  className?: string;
  children?: React.ReactNode;
};

export function Empty({ title, description, className, children }: EmptyProps) {
  return (
    <div
      className={cn(
        "flex min-h-64 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border bg-muted/35 p-8 text-center",
        className,
      )}
    >
      <div className="flex size-11 items-center justify-center rounded-md bg-background text-muted-foreground shadow-soft">
        <FileText aria-hidden="true" />
      </div>
      <div className="flex max-w-md flex-col gap-1">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
      {children ? <div className="mt-1 flex flex-wrap justify-center gap-2">{children}</div> : null}
    </div>
  );
}
