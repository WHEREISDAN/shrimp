import * as React from "react";

import { cn } from "@/lib/utils";

const fieldClasses =
  "w-full min-w-0 rounded-md border border-input bg-card text-base text-foreground shadow-xs transition-[border-color,box-shadow] outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/20 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:bg-white/[0.04]";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        fieldClasses,
        "h-(--control-h) px-2.5 [&::-webkit-search-decoration]:appearance-none",
        className,
      )}
      {...props}
    />
  );
}

export { Input, fieldClasses };
