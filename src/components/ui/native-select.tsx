import * as React from "react";
import { ChevronsUpDownIcon } from "lucide-react";

import { cn } from "@/lib/utils";

type NativeSelectProps = Omit<React.ComponentProps<"select">, "size"> & {
  /** Classes for the wrapper, which sets the width. */
  wrapperClassName?: string;
};

/** A native select (the system's own menu opens) dressed as a push button with ⌃⌄ chevrons. */
function NativeSelect({ className, wrapperClassName, ...props }: NativeSelectProps) {
  return (
    <div
      className={cn(
        "group/native-select relative w-fit has-[select:disabled]:opacity-50",
        wrapperClassName,
      )}
      data-slot="native-select-wrapper"
    >
      <select
        data-slot="native-select"
        className={cn(
          "h-(--control-h) w-full min-w-0 appearance-none truncate rounded-md border border-input bg-card py-0 pr-7 pl-2.5 text-base text-foreground shadow-xs transition-[border-color,box-shadow,background-color] outline-none hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/20 disabled:pointer-events-none dark:border-white/10 dark:bg-white/[0.06] dark:hover:bg-white/10",
          className,
        )}
        {...props}
      />
      <ChevronsUpDownIcon
        className="pointer-events-none absolute top-1/2 right-2 size-3.5 -translate-y-1/2 text-muted-foreground"
        aria-hidden="true"
        data-slot="native-select-icon"
      />
    </div>
  );
}

export { NativeSelect };
