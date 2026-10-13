import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface VibeCodingSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function VibeCodingSection({ settings, update }: VibeCodingSectionProps) {
  return (
    <section className="card stack" aria-labelledby="vibe-coding-heading">
      <h2 id="vibe-coding-heading">Vibe coding</h2>
      <ToggleRow
        label="File tagging in chat"
        description="In Cursor and Windsurf chat, say a file's name (“index dot ts”) or “tag” and its name, and Shrimp writes an @-mention. Works for files open or shown in the sidebar."
        checked={settings.vibeCoding.fileTagging}
        onChange={(fileTagging) => void update({ vibeCoding: { fileTagging } })}
      />
      <div className="toggle-text">
        <span className="card-title">Variable recognition</span>
        <p className="muted">
          In Cursor, VS Code and Windsurf, say names naturally, “user ID” for <code>userId</code>:
          Shrimp matches them to the code on screen. Say “camel case”, “snake case”, “pascal case”
          or “kebab case” before words to build a new name.
        </p>
        <ol className="intro-steps muted">
          <li>Open the Command Palette (Cmd+Shift+P).</li>
          <li>Run “Toggle Screen Reader Accessibility Mode”.</li>
          <li>Check that “Screen Reader Optimized” shows in the status bar.</li>
        </ol>
      </div>
    </section>
  );
}
