import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Group, Row, RowText, Section } from "@/components/settings";
import { NativeSelect } from "@/components/ui/native-select";
import type { AppLanguage, Settings, SettingsPatch } from "../lib/ipc";

/** Each language in its own name, so anyone can find theirs. */
const NATIVE_NAMES: Record<Exclude<AppLanguage, "system">, string> = {
  en: "English",
  de: "Deutsch",
  es: "Español",
  it: "Italiano",
  pt: "Português",
};

interface AppLanguageSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › General › App language (LANG-06). */
export function AppLanguageSection({ settings, update }: AppLanguageSectionProps) {
  const { t } = useTranslation();
  const id = useId();
  return (
    <Section headingId={`${id}-heading`} title={t("appLanguage.title")}>
      <Group>
        <Row>
          <RowText title={t("appLanguage.label")} htmlFor={id} />
          <NativeSelect
            id={id}
            wrapperClassName="min-w-40"
            value={settings.app.language}
            onChange={(event) =>
              void update({ app: { language: event.target.value as AppLanguage } })
            }
          >
            <option value="system">{t("appLanguage.system")}</option>
            {Object.entries(NATIVE_NAMES).map(([value, name]) => (
              <option key={value} value={value} lang={value}>
                {name}
              </option>
            ))}
          </NativeSelect>
        </Row>
      </Group>
    </Section>
  );
}
