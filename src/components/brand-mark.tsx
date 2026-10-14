import type { SVGProps } from "react";
import { BRAND_MARK } from "@/lib/brandMark";

/** Shrimp's mark in the current colour. */
export function BrandMark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox={`0 0 ${BRAND_MARK.size} ${BRAND_MARK.size}`}
      fill="currentColor"
      aria-hidden="true"
      {...props}
    >
      {BRAND_MARK.paths.map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}
