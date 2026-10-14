import { useId, useState, type FormEvent } from "react";
import { CircleCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Group, Row, RowText } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { deleteApiKey, errorMessage, setApiKey, type ApiKeys, type KeyAccount } from "../lib/ipc";

interface KeyRowProps {
  account: KeyAccount;
  /** The company the key belongs to, such as "OpenAI". */
  label: string;
  saved: boolean;
  onChange: (keys: ApiKeys) => void;
}

/**
 * Saves or removes an API key, in a group of its own: the key's state, then the field.
 * The key is written to the keychain and never shown again.
 */
export function KeyRow({ account, label, saved, onChange }: KeyRowProps) {
  const { t } = useTranslation();
  const id = useId();
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
    <div className="flex flex-col gap-2">
      <Group>
        <Row>
          <RowText title={t("apiKey.label", { company: label })} htmlFor={id} />
          <p
            className={
              saved
                ? "flex items-center gap-1 text-sm text-success"
                : "text-sm text-muted-foreground"
            }
          >
            {saved && <CircleCheck aria-hidden="true" className="size-3.5 shrink-0" />}
            {saved ? t("apiKey.saved") : t("apiKey.none")}
          </p>
        </Row>
        <form className="flex flex-wrap items-center gap-2 px-4 py-2.5" onSubmit={save}>
          <Input
            id={id}
            type="password"
            className="min-w-40 flex-1 basis-48"
            aria-label={t("apiKey.label", { company: label })}
            placeholder={t("apiKey.placeholder")}
            autoComplete="off"
            value={value}
            onChange={(event) => setValue(event.target.value)}
          />
          <Button type="submit" disabled={busy || value.trim() === ""}>
            {t("apiKey.save")}
          </Button>
          {saved && (
            <Button disabled={busy} onClick={() => void run(() => deleteApiKey(account))}>
              {t("apiKey.remove")}
            </Button>
          )}
        </form>
      </Group>
      {error && (
        <p className="px-1 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
