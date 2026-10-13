import { Dialog } from "./Dialog";

interface CommandModeIntroProps {
  shortcut: string;
  onDone: () => void;
  onDisable: () => void;
}

/** Shown once, when Command Mode is first turned on (CMD-06). */
export function CommandModeIntro({ shortcut, onDone, onDisable }: CommandModeIntroProps) {
  return (
    <Dialog title="Using Command Mode (beta)" onClose={onDone}>
      <ol className="intro-steps">
        <li>Select the text you want to change, or put the cursor in a text field.</li>
        <li>
          Hold <kbd>{shortcut}</kbd> and say what to do: “make this shorter”, “translate to
          Spanish”, “turn this into a list”.
        </li>
        <li>Release. The result replaces your selection; Cmd+Z undoes it.</li>
      </ol>
      <p className="muted">
        With nothing selected, ask a question and the answer appears next to the Flow Bar. Start
        with “search” to open a web search. Double-press the shortcut to keep listening hands-free;
        Esc cancels. Your selection and the field you are in are sent to your AI provider.
      </p>
      <div className="dialog-actions">
        <button type="button" onClick={onDisable}>
          Disable
        </button>
        <button type="button" className="primary" onClick={onDone} autoFocus>
          Got it
        </button>
      </div>
    </Dialog>
  );
}
