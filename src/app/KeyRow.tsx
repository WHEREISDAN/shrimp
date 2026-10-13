import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
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
      <p className={saved ? "ok" : "muted"}>{saved ? t("apiKey.saved") : t("apiKey.none")}</p>
      <form className="key-form" onSubmit={save}>
        <input
          type="password"
          aria-label={t("apiKey.label", { company: label })}
          placeholder={t("apiKey.placeholder")}
          autoComplete="off"
          value={value}
          onChange={(event) => setValue(event.target.value)}
        />
        <button type="submit" disabled={busy || value.trim() === ""}>
          {t("apiKey.save")}
        </button>
        {saved && (
          <button
            type="button"
            disabled={busy}
            onClick={() => void run(() => deleteApiKey(account))}
          >
            {t("apiKey.remove")}
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
