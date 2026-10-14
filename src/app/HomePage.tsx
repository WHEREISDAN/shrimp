import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { getAppInfo, type AppInfo } from "../lib/ipc";
import { Greeting } from "./Greeting";
import { HistorySection } from "./HistorySection";
import { ModelCard } from "./ModelCard";
import { pageColumn, PageToolbar } from "./Page";
import { PermissionsCard } from "./PermissionsCard";

type InfoState =
  { status: "loading" } | { status: "ready"; info: AppInfo } | { status: "error"; message: string };

interface HomePageProps {
  onOpenInsights: () => void;
}

export function HomePage({ onOpenInsights }: HomePageProps) {
  const { t } = useTranslation();
  return (
    <div className="app">
      <PageToolbar title={t("nav.home")} />
      <div className={cn(pageColumn(), "flex flex-col gap-7 pb-10")}>
        <Greeting onOpenInsights={onOpenInsights} />
        <ModelCard hideWhenReady />
        <PermissionsCard hideWhenGranted />
        <HistorySection />
        <AppInfoLine />
      </div>
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
        <p
          className="meta text-center text-xs text-destructive"
          role="alert"
          data-testid="app-info-error"
        >
          {state.message}
        </p>
      );
    case "ready":
      return (
        <p className="meta text-center text-xs text-muted-foreground/80" data-testid="app-info">
          {state.info.name} {state.info.version} · {state.info.os}/{state.info.arch}
        </p>
      );
  }
}
