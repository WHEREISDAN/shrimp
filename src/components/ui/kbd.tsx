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

/** A binding drawn as a key cap (the Kbd look) that is also a button. */
export const keycap =
  "inline-flex h-(--control-h-sm) min-w-10 shrink-0 items-center justify-center gap-1 rounded-md border bg-card px-2 font-sans text-sm font-medium whitespace-nowrap text-foreground tabular-nums shadow-[0_1px_0_var(--border)] transition-[background-color,border-color,color] duration-150 outline-none select-none hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring/70 dark:bg-white/[0.06] dark:hover:bg-white/10";

/** A binding while its keys are being recorded: the brand colour, as a focused field. */
export const recordingKeycap =
  "border-brand bg-brand/[0.06] text-brand-strong shadow-none ring-3 ring-brand/15 hover:bg-brand/[0.06] dark:bg-brand/10 dark:hover:bg-brand/10";

export { Kbd };
