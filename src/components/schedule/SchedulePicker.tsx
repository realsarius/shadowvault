import type { ScheduleType } from "../../store/types";
import styles from "./SchedulePicker.module.css";

interface Props {
  value: ScheduleType;
  onChange: (s: ScheduleType) => void;
}

export function SchedulePicker(props: Props) {
  const type = () => props.value.type;

  const setType = (t: ScheduleType["type"]) => {
    if (t === "Interval") props.onChange({ type: "Interval", value: { minutes: 60 } });
    else if (t === "Cron") props.onChange({ type: "Cron", value: { expression: "0 2 * * *" } });
    else if (t === "OnChange") props.onChange({ type: "OnChange" });
    else props.onChange({ type: "Manual" });
  };

  const intervalMinutes = () => (props.value.type === "Interval" ? props.value.value.minutes : 60);
  const cronExpr = () => (props.value.type === "Cron" ? props.value.value.expression : "0 2 * * *");

  return (
    <div class={styles.picker}>
      {(["Interval", "Cron", "OnChange", "Manual"] as const).map((t) => (
        <label class={styles.option}>
          <input type="radio" checked={type() === t} onChange={() => setType(t)} />
          {t === "Interval" && "Her X dakikada bir"}
          {t === "Cron" && "Cron ifadesi"}
          {t === "OnChange" && "Dosya değişince"}
          {t === "Manual" && "Sadece manuel"}
          {t === "Interval" && type() === "Interval" && (
            <input
              class={styles.numberInput}
              type="number"
              min={1}
              value={intervalMinutes()}
              onInput={(e) => props.onChange({ type: "Interval", value: { minutes: parseInt(e.currentTarget.value) || 60 } })}
            />
          )}
          {t === "Cron" && type() === "Cron" && (
            <input
              class={styles.cronInput}
              type="text"
              value={cronExpr()}
              onInput={(e) => props.onChange({ type: "Cron", value: { expression: e.currentTarget.value } })}
              placeholder="0 2 * * *"
            />
          )}
        </label>
      ))}
    </div>
  );
}
