import type { CleanupLevel } from "../lib/ipc";
import { RadioCards, type RadioCardOption } from "./RadioCards";

const OPTIONS: RadioCardOption<CleanupLevel>[] = [
  {
    value: "none",
    title: "None",
    description:
      "Pastes the raw transcript, including filler words, false starts, and spoken errors.",
  },
  { value: "light", title: "Light", description: "Removes filler words and fixes grammar." },
  {
    value: "medium",
    title: "Medium",
    description: "May reword phrases for clarity and conciseness while preserving meaning.",
  },
];

interface CleanupSectionProps {
  value: CleanupLevel;
  onChange: (level: CleanupLevel) => void;
}

export function CleanupSection({ value, onChange }: CleanupSectionProps) {
  return (
    <section className="card stack" aria-labelledby="cleanup-heading">
      <h2 id="cleanup-heading">Auto Cleanup</h2>
      <RadioCards label="Auto Cleanup" value={value} options={OPTIONS} onChange={onChange} />
    </section>
  );
}
