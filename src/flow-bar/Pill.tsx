import { useRef, type PointerEvent, type ReactNode } from "react";
import { flowBarControl, flowBarDrag, type BarControl, type DictationState } from "../lib/ipc";

interface PillProps {
  state: DictationState;
  levels: number[];
  error: boolean;
}

const send = (control: BarControl) => () => void flowBarControl(control).catch(() => undefined);

/** Movement that turns a press on the resting bar into a drag, in pixels. */
const DRAG_THRESHOLD = 4;

/** Dragging the resting bar moves it; on release it docks to the nearest edge (BAR-05). */
function useDrag() {
  const start = useRef<{ x: number; y: number } | null>(null);
  const moved = useRef(false);
  const frame = useRef(0);
  return {
    onPointerDown: (event: PointerEvent<HTMLButtonElement>) => {
      if (event.button !== 0) {
        return;
      }
      event.currentTarget.setPointerCapture(event.pointerId);
      start.current = { x: event.screenX, y: event.screenY };
      moved.current = false;
    },
    onPointerMove: (event: PointerEvent<HTMLButtonElement>) => {
      const from = start.current;
      if (!from) {
        return;
      }
      const distance = Math.hypot(event.screenX - from.x, event.screenY - from.y);
      if (!moved.current && distance < DRAG_THRESHOLD) {
        return;
      }
      moved.current = true;
      cancelAnimationFrame(frame.current);
      frame.current = requestAnimationFrame(() => void flowBarDrag(false).catch(() => undefined));
    },
    onPointerUp: () => {
      start.current = null;
      if (moved.current) {
        cancelAnimationFrame(frame.current);
        void flowBarDrag(true).catch(() => undefined);
      }
    },
    /** A press that became a drag is no click. */
    wasDrag: () => {
      const dragged = moved.current;
      moved.current = false;
      return dragged;
    },
  };
}

export function Pill({ state, levels, error }: PillProps) {
  const drag = useDrag();
  switch (state.phase) {
    case "idle":
      return (
        <button
          type="button"
          className={error ? "pill pill-idle pill-error" : "pill pill-idle"}
          aria-label="Start dictation"
          title="Click to dictate, or drag to move"
          onPointerDown={drag.onPointerDown}
          onPointerMove={drag.onPointerMove}
          onPointerUp={drag.onPointerUp}
          onClick={() => {
            if (!drag.wasDrag()) {
              send("start")();
            }
          }}
        />
      );
    case "recording":
      return (
        <div
          className={state.command ? "pill pill-recording pill-command" : "pill pill-recording"}
          role="group"
          aria-label={state.command ? "Command Mode" : "Recording"}
        >
          <ControlButton label="Cancel" onClick={send("cancel")}>
            <path d="M4 4l8 8M12 4l-8 8" />
          </ControlButton>
          <Waveform levels={levels} />
          {state.mode === "locked" ? (
            <ControlButton label="Stop" className="control-stop" onClick={send("stop")}>
              <rect x="4" y="4" width="8" height="8" rx="1.5" />
            </ControlButton>
          ) : (
            <ControlButton label="Done" onClick={send("confirm")}>
              <path d="M3.5 8.5l3 3 6-7" />
            </ControlButton>
          )}
        </div>
      );
    case "processing":
      return (
        <div className="pill pill-processing" role="status" aria-label="Transcribing">
          <span className="dot" />
          <span className="dot" />
          <span className="dot" />
        </div>
      );
  }
}

interface ControlButtonProps {
  label: string;
  className?: string;
  onClick: () => void;
  children: ReactNode;
}

function ControlButton({ label, className, onClick, children }: ControlButtonProps) {
  return (
    <button
      type="button"
      className={className ? `control ${className}` : "control"}
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        {children}
      </svg>
    </button>
  );
}

function Waveform({ levels }: { levels: number[] }) {
  return (
    <div className="waveform" aria-hidden="true">
      {levels.map((level, index) => (
        <span
          key={index}
          className="bar"
          style={{ transform: `scaleY(${Math.max(0.12, Math.min(1, level))})` }}
        />
      ))}
    </div>
  );
}
