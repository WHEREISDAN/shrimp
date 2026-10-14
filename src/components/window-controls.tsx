import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

/** Segoe Fluent Icons (Windows 11) and Segoe MDL2 Assets (Windows 10) caption glyphs. */
const GLYPH = { minimize: "", maximize: "", restore: "", close: "" };

const buttonClass =
  "grid h-full w-[46px] place-items-center font-['Segoe_Fluent_Icons','Segoe_MDL2_Assets'] text-[10px] text-foreground/85 transition-colors duration-100 outline-none hover:bg-foreground/[0.06] active:bg-foreground/[0.1] active:text-foreground/60";

/**
 * Windows' minimize, maximize/restore and close buttons for the undecorated
 * main window. They sit out of the Tab order, as native caption buttons do;
 * Alt+Space, Win+arrows, dragging and double-clicking the title area stay native.
 */
export function WindowControls() {
  const { t } = useTranslation();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const window = getCurrentWindow();
    let cancelled = false;
    const refresh = () => {
      window
        .isMaximized()
        .then((value) => !cancelled && setMaximized(value))
        .catch(() => undefined);
    };
    refresh();
    const unlisten = window.onResized(refresh);
    return () => {
      cancelled = true;
      void unlisten.then((stop) => stop());
    };
  }, []);

  const window = () => getCurrentWindow();
  return (
    <div
      role="group"
      aria-label={t("windowControls.label")}
      className="fixed top-0 right-0 z-[60] flex h-8 select-none"
    >
      <button
        type="button"
        tabIndex={-1}
        className={buttonClass}
        aria-label={t("windowControls.minimize")}
        title={t("windowControls.minimize")}
        onClick={() => void window().minimize()}
      >
        <span aria-hidden="true">{GLYPH.minimize}</span>
      </button>
      <button
        type="button"
        tabIndex={-1}
        className={buttonClass}
        aria-label={maximized ? t("windowControls.restore") : t("windowControls.maximize")}
        title={maximized ? t("windowControls.restore") : t("windowControls.maximize")}
        onClick={() => void window().toggleMaximize()}
      >
        <span aria-hidden="true">{maximized ? GLYPH.restore : GLYPH.maximize}</span>
      </button>
      <button
        type="button"
        tabIndex={-1}
        className={cn(
          buttonClass,
          "hover:bg-[#c42b1c] hover:text-white active:bg-[#c42b1c]/90 active:text-white/80",
        )}
        aria-label={t("windowControls.close")}
        title={t("windowControls.close")}
        onClick={() => void window().close()}
      >
        <span aria-hidden="true">{GLYPH.close}</span>
      </button>
    </div>
  );
}
