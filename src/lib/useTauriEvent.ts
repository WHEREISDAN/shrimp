import { useEffect, useRef } from "react";
import { onEvent } from "./ipc";

/** Tauri's unlisten is async; one that fails after teardown (the tests' mocks) is moot. */
function stopQuietly(stop: () => void) {
  const pending: unknown = stop();
  if (pending instanceof Promise) {
    pending.catch(() => undefined);
  }
}

/** Subscribes to a Tauri event for the component's lifetime. */
export function useTauriEvent<T>(
  name: string,
  handler: (payload: T) => void,
  onSubscribed?: () => void,
): void {
  const handlerRef = useRef(handler);
  const subscribedRef = useRef(onSubscribed);
  useEffect(() => {
    handlerRef.current = handler;
    subscribedRef.current = onSubscribed;
  });

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    onEvent<T>(name, (payload) => handlerRef.current(payload))
      .then((stop) => {
        if (disposed) {
          stopQuietly(stop);
          return;
        }
        unlisten = stop;
        subscribedRef.current?.();
      })
      .catch((error: unknown) => console.error(`could not listen to ${name}`, error));
    return () => {
      disposed = true;
      if (unlisten) {
        stopQuietly(unlisten);
      }
    };
  }, [name]);
}
