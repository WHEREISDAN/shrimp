import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getAppInfo, type AppInfo } from "../lib/ipc";
import { HistorySection } from "./HistorySection";
import { ModelCard } from "./ModelCard";
import { PermissionsCard } from "./PermissionsCard";
import { ShortcutsCard } from "./ShortcutsCard";

type InfoState =
  { status: "loading" } | { status: "ready"; info: AppInfo } | { status: "error"; message: string };

interface HomePageProps {
  onOpenShortcuts: () => void;
}

export function HomePage({ onOpenShortcuts }: HomePageProps) {
  const { t } = useTranslation();
  return (
    <div className="app">
      <header className="app-header">
        <h1>Shrimp</h1>
        <p className="tagline">{t("home.tagline")}</p>
      </header>
      <ModelCard />
      <PermissionsCard />
      <ShortcutsCard onOpenShortcuts={onOpenShortcuts} />
      <HistorySection />
      <AppInfoLine />
    </div>
  );
}

function AppInfoLine() {
  const [state, setState] = useState<InfoState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    getAppInfo()
      .then((info) => !cancelled && setState({ status: "ready", info }))
      .catch(
        (error: unknown) => !cancelled && setState({ status: "error", message: String(error) }),
      );
    return () => {
      cancelled = true;
    };
  }, []);

  switch (state.status) {
    case "loading":
      return <p className="meta">Loading…</p>;
    case "error":
      return (
        <p className="meta error" role="alert">
          {state.message}
        </p>
      );
    case "ready":
      return (
        <p className="meta" data-testid="app-info">
          {state.info.name} {state.info.version} · {state.info.os}/{state.info.arch}
        </p>
      );
  }
}
