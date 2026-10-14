import { useId } from "react";
import { Row, RowText } from "@/components/settings";
import { NativeSelect } from "@/components/ui/native-select";
import type { ModelOption } from "../lib/ipc";

interface ModelSelectProps {
  label: string;
  models: ModelOption[];
  value: string;
  onChange: (id: string) => void;
}

/** A model to choose: one row of a group, with the chosen model's note under its label. */
export function ModelSelect({ label, models, value, onChange }: ModelSelectProps) {
  const id = useId();
  const selected = models.find((model) => model.id === value);
  return (
    <Row>
      <RowText title={label} htmlFor={id} description={selected?.note} />
      <NativeSelect
        id={id}
        aria-label={label}
        wrapperClassName="min-w-40 max-w-full"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        {!selected && value !== "" && <option value={value}>{value}</option>}
        {models.map((model) => (
          <option key={model.id} value={model.id}>
            {model.label}
          </option>
        ))}
      </NativeSelect>
    </Row>
  );
}
