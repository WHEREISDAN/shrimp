import { useTranslation } from "react-i18next";
import { flowBarWand, type TransformSettings, type WandAction } from "../lib/ipc";

const send = (action: WandAction) => void flowBarWand(action).catch(() => undefined);

interface Choice {
  id: string;
  name: string;
}

/** Polish, Prompt Engineer and the custom transforms that can run. */
function useChoices(transforms: TransformSettings): Choice[] {
  const { t } = useTranslation();
  return [
    { id: "polish", name: t("transforms.polish.title") },
    { id: "promptEngineer", name: t("transforms.promptEngineer.title") },
    ...transforms.custom
      .filter((transform) => transform.name.trim() && transform.prompt.trim())
      .map((transform) => ({ id: transform.id, name: transform.name.trim() })),
  ];
}

interface WandProps {
  transforms: TransformSettings;
  /** The menu is open. */
  open: boolean;
  onToggle: () => void;
}

/**
 * Shown beside the resting pill on hover while Transforms are on (TRF-07):
 * the wand runs the chosen transform on the app in front, which keeps
 * focus; the expander opens the menu.
 */
export function Wand({ transforms, open, onToggle }: WandProps) {
  const { t } = useTranslation();
  const choices = useChoices(transforms);
  const chosen = choices.find((choice) => choice.id === transforms.wand) ?? choices[0];
  const run = t("wand.run", { name: chosen?.name ?? "" });
  return (
    <div className="wand" role="group" aria-label={t("wand.label")}>
      <button
        type="button"
        className="wand-run"
        aria-label={run}
        title={run}
        onClick={() => send("run")}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M3 13l6.5-6.5M11.5 2v4M9.5 4h4M5.5 2.5v2M4.5 3.5h2M12.5 9v2M11.5 10h2" />
        </svg>
      </button>
      <button
        type="button"
        className="wand-expand"
        aria-label={t("wand.choose")}
        title={t("wand.choose")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={onToggle}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M5 9.5l3-3 3 3" />
        </svg>
      </button>
    </div>
  );
}

interface WandMenuProps {
  transforms: TransformSettings;
  onClose: () => void;
}

/** Which transform the wand runs, Auto Apply after dictation, and the Transforms page. */
export function WandMenu({ transforms, onClose }: WandMenuProps) {
  const { t } = useTranslation();
  const choices = useChoices(transforms);
  const choose = (action: WandAction) => {
    send(action);
    onClose();
  };
  return (
    <div className="bar-menu wand-menu" role="menu" aria-label={t("wand.menu")}>
      {choices.map((choice) => (
        <button
          key={choice.id}
          type="button"
          role="menuitemradio"
          aria-checked={transforms.wand === choice.id}
          onClick={() => choose({ choose: choice.id })}
        >
          {choice.name}
        </button>
      ))}
      <hr className="bar-menu-separator" />
      <button
        type="button"
        role="menuitemcheckbox"
        aria-checked={transforms.autoApply !== null}
        onClick={() => choose({ autoApply: transforms.autoApply === null })}
      >
        {t("wand.autoApply")}
      </button>
      <button type="button" role="menuitem" onClick={() => choose("configure")}>
        {t("wand.configure")}
      </button>
    </div>
  );
}
