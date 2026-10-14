import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "group/button inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md border border-transparent font-medium whitespace-nowrap select-none transition-[background-color,border-color,box-shadow,color,opacity] duration-150 outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring/70 active:not-aria-[haspopup]:translate-y-px disabled:pointer-events-none disabled:opacity-45 aria-invalid:border-destructive [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-sm hover:bg-primary/88",
        outline:
          "border-input bg-card text-foreground shadow-xs hover:bg-muted aria-expanded:bg-muted dark:border-white/10 dark:bg-white/[0.06] dark:hover:bg-white/10",
        secondary: "bg-secondary text-secondary-foreground hover:bg-accent",
        ghost: "text-foreground hover:bg-accent aria-expanded:bg-accent",
        destructive:
          "bg-destructive/10 text-destructive hover:bg-destructive/15 focus-visible:outline-destructive/60 dark:bg-destructive/15 dark:hover:bg-destructive/22",
        link: "h-auto! border-0 px-0! text-brand-strong underline-offset-4 hover:underline",
      },
      size: {
        default: "h-(--control-h) px-3 text-base",
        sm: "h-(--control-h-sm) gap-1 px-2.5 text-sm [&_svg:not([class*='size-'])]:size-3.5",
        lg: "h-(--control-h-lg) gap-2 rounded-lg px-4 text-base",
        icon: "size-(--control-h)",
        "icon-sm": "size-(--control-h-sm) [&_svg:not([class*='size-'])]:size-3.5",
      },
    },
    defaultVariants: {
      variant: "outline",
      size: "default",
    },
  },
);

function Button({
  className,
  variant = "outline",
  size = "default",
  asChild = false,
  type = "button",
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  }) {
  const Comp = asChild ? Slot.Root : "button";

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      type={asChild ? undefined : type}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
