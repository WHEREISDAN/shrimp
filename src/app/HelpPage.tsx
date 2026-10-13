import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getAppInfo, getShortcuts, type ShortcutList } from "../lib/ipc";
import { MicAdviceDialog } from "./MicAdviceDialog";
import { Page } from "./Page";
import { IS_WINDOWS } from "../lib/platform";

interface HelpPageProps {
  onOpenShortcuts: () => void;
  /** Starts onboarding again. */
  onSetup?: () => void;
}

const MAC_KEYS = [
  { keys: "⌘[  ⌘]", label: "help.keyBack" },
  { keys: "⌥↑  ⌥↓", label: "help.keyPages" },
  { keys: "⌘F  /", label: "help.keySearch" },
  { keys: "↑↓  j k", label: "help.keyRows" },
  { keys: "← →", label: "help.keyTabs" },
  { keys: "⌘↩", label: "help.keySubmit" },
] as const;

const WINDOWS_KEYS = [
  { keys: "Ctrl+[  Ctrl+]", label: "help.keyBack" },
  { keys: "Alt+↑  Alt+↓", label: "help.keyPages" },
  { keys: "Ctrl+F  /", label: "help.keySearch" },
  { keys: "↑↓  j k", label: "help.keyRows" },
  { keys: "← →", label: "help.keyTabs" },
  { keys: "Ctrl+Enter", label: "help.keySubmit" },
] as const;

const KEYS = IS_WINDOWS ? WINDOWS_KEYS : MAC_KEYS;

/** Help: how to dictate, the shortcuts, keyboard navigation and setup (APP-04). */
export function HelpPage({ onOpenShortcuts, onSetup }: HelpPageProps) {
  const { t } = useTranslation();
  const [shortcuts, setShortcuts] = useState<ShortcutList | null>(null);
  const [version, setVersion] = useState("");
  const [advice, setAdvice] = useState(false);

  useEffect(() => {
    getShortcuts()
      .then(setShortcuts)
      .catch(() => undefined);
    getAppInfo()
      .then((info) => setVersion(info.version))
      .catch(() => undefined);
  }, []);

  const talk = shortcuts?.rows.find((row) => row.action === "pushToTalk")?.shortcuts[0] ?? "fn";
  return (
    <Page
      title={t("help.title")}
      modal={advice && <MicAdviceDialog onClose={() => setAdvice(false)} />}
    >
      <section className="card stack" aria-labelledby="help-start">
        <h2 id="help-start">{t("help.startTitle")}</h2>
        <ol className="intro-steps">
          <li>{t("help.start1", { shortcut: talk })}</li>
          <li>{t("help.start2", { shortcut: talk })}</li>
          <li>{t("help.start3")}</li>
          <li>{t("help.start4")}</li>
        </ol>
      </section>
      <section className="card stack" aria-labelledby="help-shortcuts">
        <div className="row">
          <h2 id="help-shortcuts">{t("help.shortcutsTitle")}</h2>
          <button type="button" onClick={onOpenShortcuts}>
            {t("help.change")}
          </button>
        </div>
        <dl className="key-list">
          {shortcuts?.rows
            .filter((row) => row.shortcuts.length > 0)
            .map((row) => (
              <div key={row.name} className="row">
                <dt>{row.name}</dt>
                <dd>
                  <kbd>{row.shortcuts.join(" / ")}</kbd>
                </dd>
              </div>
            ))}
        </dl>
      </section>
      <section className="card stack" aria-labelledby="help-keys">
        <h2 id="help-keys">{t("help.keysTitle")}</h2>
        <dl className="key-list">
          {KEYS.map((key) => (
            <div key={key.keys} className="row">
              <dt>{t(key.label)}</dt>
              <dd>
                <kbd>{key.keys}</kbd>
              </dd>
            </div>
          ))}
        </dl>
      </section>
      <section className="card stack" aria-labelledby="help-mic">
        <h2 id="help-mic">{t("help.micTitle")}</h2>
        <p className="muted">{t("help.micBody")}</p>
        <div>
          <button type="button" onClick={() => setAdvice(true)}>
            {t("help.micAdvice")}
          </button>
        </div>
      </section>
      {onSetup && (
        <section className="card stack" aria-labelledby="help-setup">
          <h2 id="help-setup">{t("help.setupTitle")}</h2>
          <p className="muted">{t("help.setupBody")}</p>
          <div>
            <button type="button" onClick={onSetup}>
              {t("help.setupAgain")}
            </button>
          </div>
        </section>
      )}
      <p className="meta">{t("help.about", { version })}</p>
    </Page>
  );
}
