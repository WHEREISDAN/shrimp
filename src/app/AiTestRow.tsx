import { useState } from "react";
import { errorMessage, testAiProvider, type AiTestResult } from "../lib/ipc";

type TestState =
  | { status: "idle" }
  | { status: "running" }
  | { status: "done"; result: AiTestResult }
  | { status: "error"; message: string };

export function AiTestRow() {
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
        Test
      </button>
      <TestOutcome state={state} />
    </div>
  );
}

function TestOutcome({ state }: { state: TestState }) {
  switch (state.status) {
    case "idle":
      return null;
    case "running":
      return <span className="muted">Testing…</span>;
    case "error":
      return (
        <span className="error" role="alert">
          {state.message}
        </span>
      );
    case "done":
      return state.result.ok ? (
        <span className="ok" role="status">
          {`Cleaned up in ${state.result.latencyMs} ms: “${state.result.output}”`}
        </span>
      ) : (
        <span className="error" role="alert">
          {state.result.message ?? "Didn't work."}
        </span>
      );
  }
}
