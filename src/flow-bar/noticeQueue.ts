import type { Notice } from "../lib/ipc";

/** The notice on screen and the ones waiting (NOT-02). */
export interface NoticeQueue {
  current: Notice | null;
  waiting: Notice[];
}

export type NoticeEvent =
  | { type: "add"; notice: Notice }
  /** The notice went away: dismissed, timed out, or its cause is gone. */
  | { type: "remove"; id: number };

export const EMPTY_QUEUE: NoticeQueue = { current: null, waiting: [] };

/** Errors interrupt whatever shows (which waits to come back); others queue. */
export function noticeQueue(state: NoticeQueue, event: NoticeEvent): NoticeQueue {
  switch (event.type) {
    case "add": {
      const { current, waiting } = state;
      if (!current) {
        return { current: event.notice, waiting };
      }
      if (event.notice.level === "error") {
        return { current: event.notice, waiting: [current, ...waiting] };
      }
      return { current, waiting: [...waiting, event.notice] };
    }
    case "remove": {
      if (state.current?.id === event.id) {
        const [next = null, ...rest] = state.waiting;
        return { current: next, waiting: rest };
      }
      return { ...state, waiting: state.waiting.filter((notice) => notice.id !== event.id) };
    }
  }
}
