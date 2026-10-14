import { getCurrentWindow, type Window } from "@tauri-apps/api/window";
import type { PointerEvent } from "react";
import { cn } from "@/lib/utils";

type ResizeDirection = Parameters<Window["startResizeDragging"]>[0];

/** Windows 11 keeps tao's frame, which resizes from outside the window everywhere but the top. */
const ONLY_FRAMELESS = "[html[data-material=mica]_&]:hidden";

const EDGES: { direction: ResizeDirection; className: string }[] = [
  { direction: "North", className: "inset-x-2.5 top-0 h-[5px] cursor-ns-resize" },
  {
    direction: "South",
    className: cn("inset-x-2.5 bottom-0 h-[5px] cursor-ns-resize", ONLY_FRAMELESS),
  },
  {
    direction: "West",
    className: cn("inset-y-2.5 left-0 w-[5px] cursor-ew-resize", ONLY_FRAMELESS),
  },
  {
    direction: "East",
    className: cn("inset-y-2.5 right-0 w-[5px] cursor-ew-resize", ONLY_FRAMELESS),
  },
  { direction: "NorthWest", className: "top-0 left-0 size-2.5 cursor-nwse-resize" },
  { direction: "NorthEast", className: "top-0 right-0 size-2.5 cursor-nesw-resize" },
  {
    direction: "SouthWest",
    className: cn("bottom-0 left-0 size-2.5 cursor-nesw-resize", ONLY_FRAMELESS),
  },
  {
    direction: "SouthEast",
    className: cn("right-0 bottom-0 size-2.5 cursor-nwse-resize", ONLY_FRAMELESS),
  },
];

function startResize(direction: ResizeDirection, event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  void getCurrentWindow()
    .startResizeDragging(direction)
    .catch(() => undefined);
}

/**
 * Resize borders just inside the undecorated main window's edges, as in
 * Electron apps. Tauri attaches its own only without its `unstable` feature,
 * which tauri-plugin-mcp turns on. A maximized window has none.
 */
export function ResizeEdges() {
  return (
    <div aria-hidden="true" className="[html[data-maximized]_&]:hidden">
      {EDGES.map(({ direction, className }) => (
        <div
          key={direction}
          data-edge={direction}
          className={cn("fixed z-[70]", className)}
          onPointerDown={(event) => startResize(direction, event)}
        />
      ))}
    </div>
  );
}
