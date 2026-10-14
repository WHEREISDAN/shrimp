import type { ComponentProps, ReactNode, Ref } from "react";
import { cn } from "@/lib/utils";

interface SectionProps extends Omit<ComponentProps<"section">, "title"> {
  title: ReactNode;
  headingId: string;
  headingRef?: Ref<HTMLHeadingElement> | undefined;
  /** The heading takes focus when the section is revealed. */
  focusableHeading?: boolean | undefined;
  /** Controls beside the heading, such as a page's search field. */
  action?: ReactNode | undefined;
}

/**
 * A titled block of a page, as in macOS System Settings: the heading sits above
 * its group, and footnotes below it.
 */
export function Section({
  title,
  headingId,
  headingRef,
  focusableHeading = false,
  action,
  className,
  children,
  ...props
}: SectionProps) {
  return (
    <section
      aria-labelledby={headingId}
      className={cn("flex flex-col gap-2", className)}
      {...props}
    >
      <div className="flex min-h-6 flex-wrap items-end justify-between gap-x-3 gap-y-2 px-1">
        <h2
          id={headingId}
          ref={headingRef}
          tabIndex={focusableHeading ? -1 : undefined}
          className="text-base font-semibold outline-none"
        >
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}

/** Rows grouped in one rounded, hairline-bordered box (an inset grouped list). */
export function Group({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="group"
      className={cn(
        "flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs",
        className,
      )}
      {...props}
    />
  );
}

/** One row of a group: its text on the left, its control on the right. */
export function Row({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="row"
      className={cn(
        "flex min-h-(--row-min-h) flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 py-2.5",
        className,
      )}
      {...props}
    />
  );
}

interface RowTextProps {
  title: ReactNode;
  description?: ReactNode | undefined;
  /** Associates the title with a control, as its label. */
  htmlFor?: string | undefined;
  descriptionId?: string | undefined;
  className?: string | undefined;
}

export function RowText({ title, description, htmlFor, descriptionId, className }: RowTextProps) {
  return (
    <div className={cn("flex min-w-0 flex-1 basis-56 flex-col gap-0.5", className)}>
      {htmlFor ? (
        <label htmlFor={htmlFor} className="text-base">
          {title}
        </label>
      ) : (
        <p className="text-base">{title}</p>
      )}
      {description && (
        <p id={descriptionId} className="text-sm text-muted-foreground">
          {description}
        </p>
      )}
    </div>
  );
}

/** A footnote under a group. */
export function Note({ className, ...props }: ComponentProps<"p">) {
  return <p className={cn("px-1 text-sm text-muted-foreground", className)} {...props} />;
}
