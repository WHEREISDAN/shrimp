import { useLayoutEffect, type RefObject } from "react";
import { flowBarLayout } from "../lib/ipc";

/**
 * Reports the Flow Bar's content size to Rust, which resizes the window to
 * match, so transparent margins never sit over other apps' content.
 */
export function useContentSize(ref: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) {
      return;
    }
    let last = "";
    let frame = 0;
    const report = () => {
      const { width, height } = element.getBoundingClientRect();
      const size = { width: Math.ceil(width), height: Math.ceil(height) };
      const key = `${size.width}x${size.height}`;
      if (key === last || size.width === 0 || size.height === 0) {
        return;
      }
      last = key;
      void flowBarLayout(size.width, size.height).catch(() => undefined);
    };
    report();
    if (typeof ResizeObserver === "undefined") {
      return;
    }
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(report);
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [ref]);
}
