import { useCallback, useLayoutEffect, useRef, type RefObject } from "react";
import { flowBarLayout } from "../lib/ipc";

/**
 * Reports the Flow Bar's content size to Rust, which resizes the window to
 * match, so transparent margins never sit over other apps' content.
 */
export function useContentSize(ref: RefObject<HTMLElement | null>): void {
  const last = useRef("");
  const report = useCallback(() => {
    const element = ref.current;
    if (!element) {
      return;
    }
    const { width, height } = element.getBoundingClientRect();
    const size = { width: Math.ceil(width), height: Math.ceil(height) };
    const key = `${size.width}x${size.height}`;
    // 0 x 0 hides the window: nothing to show (BAR-04).
    if (key === last.current) {
      return;
    }
    last.current = key;
    void flowBarLayout(size.width, size.height).catch(() => undefined);
  }, [ref]);

  // A hidden panel gets no rendering updates, so no ResizeObserver callbacks:
  // measuring after every render lets new content show it again.
  useLayoutEffect(report);

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === "undefined") {
      return;
    }
    let frame = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(report);
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [ref, report]);
}
