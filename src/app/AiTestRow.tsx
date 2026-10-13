import { useState } from "react";
import { useTranslation } from "react-i18next";
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
    <div className="inline-row">
      <button type="button" onClick={run} disabled={state.status === "running"}>
        {t("aiProvider.test")}
      </button>
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
      return <span className="muted">{t("aiProvider.testing")}</span>;
    case "error":
      return (
        <span className="error" role="alert">
          {state.message}
        </span>
      );
    case "done":
      return state.result.ok ? (
        <span className="ok" role="status">
          {t("aiProvider.testOk", {
            latency: state.result.latencyMs,
            output: state.result.output,
          })}
        </span>
      ) : (
        <span className="error" role="alert">
          {state.result.message ?? t("aiProvider.testFailed")}
        </span>
      );
  }
}
