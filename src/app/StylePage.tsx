import { CleanupSection } from "./CleanupSection";
import { Page } from "./Page";
import { StylesSection } from "./StylesSection";
import { useSettings } from "./useSettings";

export function StylePage() {
  const { settings, error, update } = useSettings();

  return (
    <Page title="Style">
      {error && (
        <p className="error" role="alert">
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
        </>
      ) : (
        !error && <p className="muted">Loading…</p>
      )}
    </Page>
  );
}
