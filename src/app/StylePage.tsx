import { useTranslation } from "react-i18next";
import { AppFormatsSection } from "./AppFormatsSection";
import { CleanupSection } from "./CleanupSection";
import { Page } from "./Page";
import { StylesSection } from "./StylesSection";
import { useSettings } from "./useSettings";

export function StylePage() {
  const { t } = useTranslation();
  const { settings, error, update } = useSettings();

  return (
    <Page title={t("style.title")}>
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      {settings ? (
        <>
          <CleanupSection
            value={settings.ai.cleanup}
            onChange={(cleanup) => void update({ ai: { cleanup } })}
          />
          <StylesSection
            styles={settings.styles}
            onChange={(category, style) => void update({ styles: { [category]: style } })}
          />
          <AppFormatsSection
            formats={settings.appFormats}
            onChange={(appFormats) => void update({ appFormats })}
          />
        </>
      ) : (
        !error && <p className="text-muted-foreground">{t("common.loading")}</p>
      )}
    </Page>
  );
}
