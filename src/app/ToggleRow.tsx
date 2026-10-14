import { useId } from "react";
import { Row, RowText } from "@/components/settings";
import { Switch } from "@/components/ui/switch";

interface ToggleRowProps {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

/** A setting that is on or off, with its explanation: one row of a group. */
export function ToggleRow({ label, description, checked, disabled, onChange }: ToggleRowProps) {
  const id = useId();
  const descriptionId = description ? `${id}-description` : undefined;
  return (
    <Row className="toggle-row flex-nowrap">
      <RowText title={label} htmlFor={id} description={description} descriptionId={descriptionId} />
      <Switch
        id={id}
        checked={checked}
        disabled={disabled}
        aria-describedby={descriptionId}
        onCheckedChange={onChange}
      />
    </Row>
  );
}
