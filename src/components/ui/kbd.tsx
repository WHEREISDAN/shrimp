import * as React from "react";

import { cn } from "@/lib/utils";

/** A key cap: hairline border with a 1 px bottom edge, as on Apple's keyboards. */
function Kbd({ className, ...props }: React.ComponentProps<"kbd">) {
  return (
    <kbd
      data-slot="kbd"
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center gap-0.5 rounded-[5px] border border-border bg-card px-1.5 align-[1px] font-sans text-xs font-medium whitespace-nowrap text-foreground/80 shadow-[0_1px_0_var(--border)] select-none dark:bg-white/[0.06]",
        className,
      )}
      {...props}
    />
  );
}

export { Kbd };
