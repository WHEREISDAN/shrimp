import { useEffect, useState } from "react";
import { getAppInfo, type AppInfo } from "../lib/ipc";
import { Greeting } from "./Greeting";
import { HistorySection } from "./HistorySection";
import { ModelCard } from "./ModelCard";
import { PermissionsCard } from "./PermissionsCard";

type InfoState =
  { status: "loading" } | { status: "ready"; info: AppInfo } | { status: "error"; message: string };

interface HomePageProps {
  onOpenInsights: () => void;
}

export function HomePage({ onOpenInsights }: HomePageProps) {
  return (
    <div className="app">
      <Greeting onOpenInsights={onOpenInsights} />
      <ModelCard hideWhenReady />
      <PermissionsCard hideWhenGranted />
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
      return null;
    case "error":
      return (
        <p className="meta error" role="alert" data-testid="app-info-error">
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
