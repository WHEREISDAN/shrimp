import { useTranslation } from "react-i18next";
import type { CleanupLevel } from "../lib/ipc";
import { RadioCards, type RadioCardOption } from "./RadioCards";

interface CleanupSectionProps {
  value: CleanupLevel;
  onChange: (level: CleanupLevel) => void;
}

export function CleanupSection({ value, onChange }: CleanupSectionProps) {
  const { t } = useTranslation();
  const options: RadioCardOption<CleanupLevel>[] = [
    { value: "none", title: t("style.cleanup.none"), description: t("style.cleanup.noneBody") },
    { value: "light", title: t("style.cleanup.light"), description: t("style.cleanup.lightBody") },
    {
      value: "medium",
      title: t("style.cleanup.medium"),
      description: t("style.cleanup.mediumBody"),
    },
  ];
  return (
    <section className="card stack" aria-labelledby="cleanup-heading">
      <h2 id="cleanup-heading">{t("style.cleanup.title")}</h2>
      <RadioCards
        label={t("style.cleanup.title")}
        value={value}
        options={options}
        onChange={onChange}
      />
    </section>
  );
}
