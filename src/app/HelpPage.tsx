import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Row, RowText, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
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
      <Section headingId="help-start" title={t("help.startTitle")}>
        <Group>
          <ol className="m-0 flex list-none flex-col divide-y divide-separator p-0">
            {[
              t("help.start1", { shortcut: talk }),
              t("help.start2", { shortcut: talk }),
              t("help.start3"),
              t("help.start4"),
            ].map((step, index) => (
              <li key={index} className="flex items-start gap-3 px-4 py-3">
                <span
                  aria-hidden="true"
                  className="grid size-5 shrink-0 place-items-center rounded-full bg-brand/12 text-xs font-semibold text-brand-strong tabular-nums"
                >
                  {index + 1}
                </span>
                <span>{step}</span>
              </li>
            ))}
          </ol>
        </Group>
      </Section>
      <Section
        headingId="help-shortcuts"
        title={t("help.shortcutsTitle")}
        action={
          <Button size="sm" onClick={onOpenShortcuts}>
            {t("help.change")}
          </Button>
        }
      >
        <Group>
          <dl className="m-0 flex flex-col divide-y divide-separator">
            {shortcuts?.rows
              .filter((row) => row.shortcuts.length > 0)
              .map((row) => (
                <Row key={row.name} className="min-h-10 py-2">
                  <dt>{row.name}</dt>
                  <dd className="m-0">
                    <Kbd>{row.shortcuts.join(" / ")}</Kbd>
                  </dd>
                </Row>
              ))}
          </dl>
        </Group>
      </Section>
      <Section headingId="help-keys" title={t("help.keysTitle")}>
        <Group>
          <dl className="m-0 flex flex-col divide-y divide-separator">
            {KEYS.map((key) => (
              <Row key={key.keys} className="min-h-10 py-2">
                <dt>{t(key.label)}</dt>
                <dd className="m-0">
                  <Kbd>{key.keys}</Kbd>
                </dd>
              </Row>
            ))}
          </dl>
        </Group>
      </Section>
      <Section headingId="help-mic" title={t("help.micTitle")}>
        <Group>
          <Row>
            <RowText title={t("help.micBody")} />
            <Button size="sm" onClick={() => setAdvice(true)}>
              {t("help.micAdvice")}
            </Button>
          </Row>
        </Group>
      </Section>
      {onSetup && (
        <Section headingId="help-setup" title={t("help.setupTitle")}>
          <Group>
            <Row>
              <RowText title={t("help.setupBody")} />
              <Button size="sm" onClick={onSetup}>
                {t("help.setupAgain")}
              </Button>
            </Row>
          </Group>
        </Section>
      )}
      <Note className="meta text-center text-xs">{t("help.about", { version })}</Note>
    </Page>
  );
}
