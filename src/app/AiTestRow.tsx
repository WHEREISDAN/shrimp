import { useState } from "react";
import { CircleCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { errorMessage, testAiProvider, type AiTestResult } from "../lib/ipc";

type TestState =
  | { status: "idle" }
  | { status: "running" }
  | { status: "done"; result: AiTestResult }
  | { status: "error"; message: string };

export function AiTestRow() {
  const { t } = useTranslation();
  const [state, setState] = useState<TestState>({ status: "idle" });

  const run = () => {
    setState({ status: "running" });
    testAiProvider()
      .then((result) => setState({ status: "done", result }))
      .catch((err: unknown) => setState({ status: "error", message: errorMessage(err) }));
  };

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 px-1">
      <Button size="sm" onClick={run} disabled={state.status === "running"}>
        {t("aiProvider.test")}
      </Button>
      <TestOutcome state={state} />
    </div>
  );
}

function TestOutcome({ state }: { state: TestState }) {
  const { t } = useTranslation();
  switch (state.status) {
    case "idle":
      return null;
    case "running":
      return <span className="text-sm text-muted-foreground">{t("aiProvider.testing")}</span>;
    case "error":
      return (
        <span className="text-sm text-destructive" role="alert">
          {state.message}
        </span>
      );
    case "done":
      return state.result.ok ? (
        <span className="flex items-start gap-1 text-sm text-success tabular-nums" role="status">
          <CircleCheck aria-hidden="true" className="mt-px size-3.5 shrink-0" />
          {t("aiProvider.testOk", {
            latency: state.result.latencyMs,
            output: state.result.output,
          })}
        </span>
      ) : (
        <span className="text-sm text-destructive" role="alert">
          {state.result.message ?? t("aiProvider.testFailed")}
        </span>
      );
  }
}
