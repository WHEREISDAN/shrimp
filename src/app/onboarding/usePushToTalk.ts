import { useCallback, useEffect, useState } from "react";
import { EVENTS, getShortcuts } from "../../lib/ipc";
import { useTauriEvent } from "../../lib/useTauriEvent";

/** The first push-to-talk and hands-free shortcuts, kept current. */
export function usePushToTalk() {
  const [shortcuts, setShortcuts] = useState({ talk: "fn", handsFree: "fn Space" });
  const load = useCallback(() => {
    getShortcuts()
      .then((list) => {
        const first = (action: string) =>
          list.rows.find((row) => row.action === action)?.shortcuts[0];
        setShortcuts({
          talk: first("pushToTalk") ?? "fn",
          handsFree: first("handsFree") ?? "",
        });
      })
      .catch(() => undefined);
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);
  return shortcuts;
}
