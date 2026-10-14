import * as React from "react";
import { Switch as SwitchPrimitive } from "radix-ui";

import { cn } from "@/lib/utils";

/** Radix's switch with Apple's proportions: a coral track and a white thumb on a spring. */
function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "peer relative inline-flex h-5 w-[34px] shrink-0 items-center rounded-full p-0.5 shadow-[inset_0_0_0_0.5px_rgb(0_0_0/0.06)] transition-colors duration-200 outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring/70 disabled:cursor-not-allowed disabled:opacity-45 data-checked:bg-brand data-unchecked:bg-foreground/15",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="pointer-events-none block size-4 rounded-full bg-white shadow-[0_1px_2px_rgb(0_0_0/0.22),0_0_0_0.5px_rgb(0_0_0/0.04)] transition-transform duration-250 ease-spring data-checked:translate-x-[14px] data-unchecked:translate-x-0"
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
