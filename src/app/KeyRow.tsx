import { useState, type FormEvent } from "react";
import { deleteApiKey, errorMessage, setApiKey, type ApiKeys, type KeyAccount } from "../lib/ipc";

interface KeyRowProps {
  account: KeyAccount;
  /** The company the key belongs to, such as "OpenAI". */
  label: string;
  saved: boolean;
  onChange: (keys: ApiKeys) => void;
}

/** Saves or removes an API key. The key is written to the keychain and never shown again. */
export function KeyRow({ account, label, saved, onChange }: KeyRowProps) {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async (action: () => Promise<ApiKeys>, done?: () => void) => {
    setBusy(true);
    setError(null);
    try {
      onChange(await action());
      done?.();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const save = (event: FormEvent) => {
    event.preventDefault();
    const key = value.trim();
    if (key) {
      void run(
        () => setApiKey(account, key),
        () => setValue(""),
      );
    }
  };

  return (
    <div className="key-row">
      <p className={saved ? "ok" : "muted"}>
        {saved ? "Key saved in your keychain" : "No key yet"}
      </p>
      <form className="key-form" onSubmit={save}>
        <input
          type="password"
          aria-label={`${label} API key`}
          placeholder="Paste API key"
          autoComplete="off"
          value={value}
          onChange={(event) => setValue(event.target.value)}
        />
        <button type="submit" disabled={busy || value.trim() === ""}>
          Save key
        </button>
        {saved && (
          <button
            type="button"
            disabled={busy}
            onClick={() => void run(() => deleteApiKey(account))}
          >
            Remove key
          </button>
        )}
      </form>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
